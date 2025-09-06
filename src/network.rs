pub mod node;
pub mod stream;

use std::collections::{BTreeSet, HashMap};

use futures::{StreamExt, channel::mpsc, future, never::Never};
use iroh::{Endpoint, NodeAddr, NodeId, RelayUrl, Watcher, endpoint::Incoming};
use stream::Stream;

use crate::identity::Keys;

pub const ALPN: &[u8] = b"/freecord/1";

#[derive(Debug)]
pub enum Message {
    NodeConnected(NodeId),
    NodeError(node::Error),
    NetworkCreated(Network),
    Error(Error),
}

pub enum Input {
    ConnectionAccepted(Incoming),
    /// Attempt connecting to specified node.
    Connect(NodeId, RelayUrl),
    Close,
}

#[derive(Debug, Clone)]
pub struct Network {
    pub input_sender: mpsc::Sender<Input>,
    pub home_relay: RelayUrl,
}

async fn run(stream: Box<Stream>, sender: mpsc::UnboundedSender<Message>) -> Never {
    let endpoint = match create_endpoint(&stream.keys).await {
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

    let (input_sender, input_receiver) = mpsc::channel(100);

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

                                connections.insert(node_id.clone(), connection);

                                Some(Message::NodeConnected(node_id))
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

                    connections.insert(node_id.clone(), connection);

                    Some(Message::NodeConnected(node_id))
                }
                Err(error) => {
                    log::warn!("[network] Error during connection attempt to {node_id}: {error}");

                    Some(Message::NodeError(error.into()))
                }
            },
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

async fn create_endpoint(keys: &Keys) -> Result<Endpoint, Error> {
    log::info!("[network] Creating endpoind...");

    let endpoint = Endpoint::builder()
        .secret_key(keys.private.clone().into())
        .alpns(vec![ALPN.to_vec()])
        .bind()
        .await?;

    endpoint.home_relay().initialized().await;

    Ok(endpoint)
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    BindError(#[from] iroh::endpoint::BindError),
}
