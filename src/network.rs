pub mod channel;
pub mod node;
pub mod stream;

use std::collections::{BTreeSet, HashMap};

use ed25519_dalek::SigningKey;
use futures::{StreamExt, channel::mpsc, future, never::Never};
use iroh::{
    Endpoint, NodeAddr, NodeId, RelayUrl, Watcher,
    endpoint::{Connection, Incoming},
};
use stream::Stream;

use node::{Receiver, Sender};

use crate::node::{ConnectionState, Direction};

pub const ALPN: &[u8] = b"/freecord/1";

#[derive(Debug)]
pub enum Message {
    NodeConnected(NodeId, ConnectionState, Receiver),
    NodeError(node::Error),
    NetworkCreated(Network),
    Error(Error),
}

pub enum Input {
    ConnectionAccepted(Incoming),
    /// Attempt connecting to specified node.
    Connect(NodeId, RelayUrl),
    NodeDisconnected(NodeId),
    Close,
}

#[derive(Debug, Clone)]
pub struct Network {
    pub input_sender: mpsc::UnboundedSender<Input>,
    pub home_relay: RelayUrl,
}

async fn run(stream: Box<Stream>, sender: mpsc::UnboundedSender<Message>) -> Never {
    let private_key = stream.keys.private.clone();

    let endpoint = match create_endpoint(private_key.clone()).await {
        Ok(endpoint) => endpoint,
        Err(error) => {
            let _ = sender.unbounded_send(Message::Error(error));
            // Wait until backend drops the stream.
            future::pending().await
        }
    };

    let home_relay = endpoint
        .home_relay()
        .get()
        .first()
        .expect("expected to have a valid home relay")
        .clone();

    log::info!("[network] Endpoint home relay set to: {home_relay}");

    let (input_sender, input_receiver) = mpsc::unbounded();

    let _ = sender.unbounded_send(Message::NetworkCreated(Network {
        input_sender,
        home_relay,
    }));

    let mut connections = HashMap::new();

    let mut input_streams = futures::stream::select(
        futures::stream::unfold((), |()| {
            let endpoint = endpoint.clone();
            Box::pin(async move {
                match endpoint.accept().await {
                    Some(incoming) => Some((Input::ConnectionAccepted(incoming), ())),
                    None => None,
                }
            })
        }),
        input_receiver,
    );

    loop {
        let response = match input_streams.select_next_some().await {
            Input::ConnectionAccepted(incoming) => {
                let remote_address = incoming.remote_address();

                log::info!("[network] Received a connection request from {remote_address}");

                match incoming.accept() {
                    Ok(connecting) => {
                        log::info!(
                            "[network] Successfully accepted connection request from \
                             {remote_address}"
                        );

                        match connecting.await {
                            Ok(connection) => {
                                let node_id = connection.remote_node_id().expect(
                                    "expected a valid connection to havea valid remote node id",
                                );

                                log::info!(
                                    "[network] Successfully established a connection to node \
                                     {node_id} through {remote_address}"
                                );

                                // Open bidirectional channel to remote peer.
                                let (send, receive) =
                                    accept_bi_connection(&connection).await.unwrap();

                                let remote_info = endpoint.remote_info(node_id).unwrap();

                                connections.insert(node_id.clone(), connection);

                                Some(Message::NodeConnected(
                                    node_id,
                                    ConnectionState::Connected {
                                        direction: Direction::Inbound,
                                        node_info: remote_info,
                                        sender: send,
                                    },
                                    receive,
                                ))
                            }
                            Err(error) => {
                                log::warn!(
                                    "[network] Error during connection attempt to \
                                     {remote_address}: {error}"
                                );

                                Some(Message::NodeError(error.into()))
                            }
                        }
                    }
                    Err(error) => {
                        log::warn!(
                            "[network] Could not accept connection to {remote_address}: {error}"
                        );
                        None
                    }
                }
            }
            Input::Connect(node_id, home_relay) => match endpoint
                .connect(
                    NodeAddr {
                        node_id: node_id.clone(),
                        relay_url: Some(home_relay),
                        direct_addresses: BTreeSet::new(),
                    },
                    ALPN,
                )
                .await
            {
                Ok(connection) => {
                    log::info!("[network] Successfully established a connection to node {node_id}");

                    // Open bidirectional channel to remote peer.
                    let (send, receive) = open_bi_connection(&connection).await.unwrap();

                    let remote_info = endpoint.remote_info(node_id).unwrap();

                    connections.insert(node_id.clone(), connection);

                    Some(Message::NodeConnected(
                        node_id,
                        ConnectionState::Connected {
                            direction: Direction::Outbound,
                            node_info: remote_info,
                            sender: send,
                        },
                        receive,
                    ))
                }
                Err(error) => {
                    log::warn!("[network] Error during connection attempt to {node_id}: {error}");

                    Some(Message::NodeError(error.into()))
                }
            },
            Input::NodeDisconnected(node_id) => {
                log::info!("[network] {node_id} has disconnected");

                connections.remove(&node_id);

                None
            }
            Input::Close => {
                // Wait until backend drops the stream.
                future::pending().await
            }
        };

        if let Some(response) = response {
            let _ = sender.unbounded_send(response);
        }
    }
}

async fn create_endpoint(private_key: SigningKey) -> Result<Endpoint, Error> {
    log::info!("[network] Creating endpoint...");

    let endpoint = Endpoint::builder()
        .secret_key(private_key.into())
        .alpns(vec![ALPN.to_vec()])
        .bind()
        .await?;

    endpoint.home_relay().initialized().await;

    Ok(endpoint)
}

async fn open_bi_connection(connection: &Connection) -> Result<(Sender, Receiver), Error> {
    let node_id = connection.remote_node_id()?;

    log::info!("[network] Opening communications to {node_id}");

    let (mut send, mut receive) = channel::open_bi(&connection).await?;

    send.send(node::Message::Hello).await?;

    if !matches!(receive.recv().await?, Some(node::Message::Hello)) {
        log::error!("[network] Received invalid message from {node_id} during handshake");
        return Err(Error::InvalidHandshakeReply);
    }

    log::info!("[network] Successfully shook hands with {node_id}");
    log::info!("[network] Successfully opened communications to {node_id}");

    Ok((send, receive))
}

async fn accept_bi_connection(connection: &Connection) -> Result<(Sender, Receiver), Error> {
    let node_id = connection.remote_node_id()?;

    log::info!("[network] Awaiting handshake from {node_id}");

    let (mut send, mut receive) = channel::accept_bi(&connection).await?;

    if !matches!(receive.recv().await?, Some(node::Message::Hello)) {
        log::error!("[network] Received invalid message from {node_id} during handshake");
        return Err(Error::InvalidHandshakeReply);
    }

    send.send(node::Message::Hello).await?;

    log::info!("[network] Successfully shook hands with {node_id}");
    log::info!("[network] Successfully opened communications to {node_id}");

    Ok((send, receive))
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Received invalid handshake reply from remote node")]
    InvalidHandshakeReply,
    #[error(transparent)]
    Bind(#[from] iroh::endpoint::BindError),
    #[error(transparent)]
    Connection(#[from] iroh::endpoint::ConnectionError),
    #[error(transparent)]
    RemoteNodeId(#[from] iroh::endpoint::RemoteNodeIdError),
    #[error(transparent)]
    Send(#[from] channel::SendError),
    #[error(transparent)]
    Recv(#[from] channel::RecvError),
}
