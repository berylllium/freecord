pub mod channel;
pub mod node;

use std::{
    collections::{BTreeSet, HashMap},
    pin::Pin,
};

use ed25519_dalek::SigningKey;
use futures::{SinkExt, StreamExt, channel::mpsc, future, stream::FusedStream};
use iced::Task;
use iroh::{
    Endpoint, NodeAddr, NodeId, RelayUrl, Watcher,
    endpoint::{Connection, Incoming, RemoteInfo},
};

use node::{Receiver, Sender};
use tokio::task::JoinHandle;

use crate::node::{ConnectionState, Direction};

pub const ALPN: &[u8] = b"/freecord/1";

#[derive(Debug)]
pub enum Message {
    NodeConnected(NodeId, Direction, RemoteInfo, Sender, Receiver),
    NodeError(node::Error),
    NetworkCreated(Network),
    NetworkClosed,
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

enum State {
    Disconnected,
    Connected {
        endpoint: Endpoint,
        input_streams: Pin<Box<dyn FusedStream<Item = Input> + Send>>,
        connections: HashMap<NodeId, Connection>,
    },
    PreQuit,
    Quit,
}

pub fn spawn(private_key: SigningKey) -> JoinHandle<()> {
    let (sender, receiver) = mpsc::channel(100);

    tokio::spawn(task(private_key, sender))
}

async fn task(private_key: SigningKey, mut sender: mpsc::Sender<Message>) {
    // We use an option here sp we never leave the binding in an uninitialized state.
    let mut state = Some(State::Disconnected);

    loop {
        let next_state = state.take().unwrap().step(&private_key).await;

        if let Some(next_state) = next_state {
            if let Some(message) = next_state.0 {
                sender.send(message).await;

                state = Some(next_state.1);
            }
        } else {
            return;
        }
    }
}

impl State {
    /// Take a step forward.
    async fn step(mut self, private_key: &SigningKey) -> Option<(Option<Message>, State)> {
        match self {
            State::Disconnected => {
                let endpoint = match create_endpoint(private_key).await {
                    Ok(endpoint) => endpoint,
                    Err(err) => return Some((Some(err.into()), State::Quit)),
                };

                let home_relay = endpoint
                    .home_relay()
                    .get()
                    .first()
                    .expect("expected to have a valid home relay")
                    .clone();

                log::info!("[network] Endpoint home relay set to: {home_relay}");

                let (input_sender, input_receiver) = mpsc::unbounded();

                let input_streams = futures::stream::select(
                    futures::stream::unfold((), {
                        let endpoint = endpoint.clone();
                        move |()| {
                            let endpoint = endpoint.clone();
                            async move {
                                match endpoint.accept().await {
                                    Some(incoming) => {
                                        Some((Input::ConnectionAccepted(incoming), ()))
                                    }
                                    None => None,
                                }
                            }
                        }
                    }),
                    input_receiver,
                )
                .fuse();

                Some((
                    Some(Message::NetworkCreated(Network {
                        input_sender,
                        home_relay,
                    })),
                    State::Connected {
                        endpoint,
                        input_streams: Box::pin(input_streams),
                        connections: HashMap::new(),
                    },
                ))
            }
            State::Connected {
                ref endpoint,
                ref mut input_streams,
                ref mut connections,
            } => match input_streams.select_next_some().await {
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

                                    Some((
                                        Some(Message::NodeConnected(
                                            node_id,
                                            Direction::Inbound,
                                            remote_info,
                                            send,
                                            receive,
                                        )),
                                        self,
                                    ))
                                }
                                Err(error) => {
                                    log::warn!(
                                        "[network] Error during connection attempt to \
                                     {remote_address}: {error}"
                                    );

                                    Some((Some(Message::NodeError(error.into())), self))
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
                        log::info!(
                            "[network] Successfully established a connection to node {node_id}"
                        );

                        // Open bidirectional channel to remote peer.
                        let (send, receive) = open_bi_connection(&connection).await.unwrap();

                        let remote_info = endpoint.remote_info(node_id).unwrap();

                        connections.insert(node_id.clone(), connection);

                        Some((
                            Some(Message::NodeConnected(
                                node_id,
                                Direction::Outbound,
                                remote_info,
                                send,
                                receive,
                            )),
                            self,
                        ))
                    }
                    Err(error) => {
                        log::warn!(
                            "[network] Error during connection attempt to {node_id}: {error}"
                        );

                        Some((Some(Message::NodeError(error.into())), self))
                    }
                },
                Input::NodeDisconnected(node_id) => {
                    log::info!("[network] {node_id} has disconnected");

                    connections.remove(&node_id);

                    Some((None, self))
                }
                Input::Close => {
                    log::info!("[network] Closing network...");
                    // Wait until backend drops the stream.
                    connections.clear();
                    endpoint.close().await;

                    Some((None, State::PreQuit))
                }
            },
            State::PreQuit => Some((Some(Message::NetworkClosed), State::Quit)),
            State::Quit => None,
        }
    }
}

// pub fn create_task(private_key: SigningKey) -> Task<Message> {
//     Task::stream(
//         futures::stream::unfold(State::Disconnected, move |state| {
//             let private_key = private_key.clone();
//             async move { state.step(private_key).await }
//         })
//         .filter_map(|msg| async move { msg }),
//     )
// }

async fn create_endpoint(private_key: &SigningKey) -> Result<Endpoint, Error> {
    log::info!("[network] Creating endpoint...");

    let endpoint = Endpoint::builder()
        .secret_key(private_key.clone().into())
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

impl From<Error> for Message {
    fn from(value: Error) -> Self {
        Message::Error(value)
    }
}
