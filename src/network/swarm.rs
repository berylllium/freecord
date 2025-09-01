use std::time::Duration;

use futures::{StreamExt, channel::mpsc, future, never::Never, stream};
use iced::advanced::{
    graphics::futures::BoxStream,
    subscription::{EventStream, Hasher, Recipe},
};
use libp2p::{
    Multiaddr, PeerId, SwarmBuilder,
    core::ConnectedPoint,
    dcutr, identify,
    identity::{Keypair, ed25519},
    multiaddr::Protocol,
    noise, ping,
    swarm::{DialError, NetworkBehaviour, SwarmEvent},
    tcp, yamux,
};
use tokio::time::Instant;

use crate::{config, identity};

type Swarm = libp2p::Swarm<Behaviour>;

#[derive(Clone, Debug)]
pub enum Message {
    NodeConnected(PeerId),
    NodeDisconnected(PeerId),
    SwarmCreated(mpsc::Sender<Input>),
    SwarmCreationError(Error),
}

pub enum Input {
    /// An input from the swarm.
    Swarm(SwarmEvent<BehaviourEvent>),
    /// Indicates to the stream to stop doing what its doing and await for the backend to drop the
    /// stream.
    Close,
}

/// The swarm stream for the app. All p2p networking goes through this swarm.
#[derive(Clone, Hash)]
pub struct Stream {
    pub keys: identity::Keys,
    pub config: config::Network,
    pub nodes: config::NodeMap,
}

impl Recipe for Stream {
    type Output = Message;

    fn hash(&self, state: &mut Hasher) {
        use std::hash::Hash;
        Hash::hash(self, state);
    }

    fn stream(self: Box<Self>, _input: EventStream) -> BoxStream<Self::Output> {
        let (sender, receiver) = mpsc::unbounded();

        let runner =
            stream::once(async { tokio::spawn(run(self, sender)).await }).map(|_| unreachable!());

        stream::select(receiver, runner).boxed()
    }
}

#[allow(unused_variables)]
async fn run(stream: Box<Stream>, sender: mpsc::UnboundedSender<Message>) -> Never {
    let mut swarm = match create_swarm(&stream.keys) {
        Ok(swarm) => swarm,
        Err(err) => {
            let _ = sender.unbounded_send(Message::SwarmCreationError(err));
            // Wait until backend drops stream.
            future::pending::<Swarm>().await
        }
    };

    let relay_address = if let Some(relay_address) = &stream.config.relay_address {
        if let Ok(relay_address) = Multiaddr::try_from(relay_address.as_str()) {
            relay_address
        } else {
            let _ = sender.unbounded_send(Message::SwarmCreationError(Error::MissingRelayAddress));
            // Wait until backend drops stream.
            future::pending::<Multiaddr>().await
        }
    } else {
        let _ = sender.unbounded_send(Message::SwarmCreationError(Error::MissingRelayAddress));
        // Wait until backend drops stream.
        future::pending::<Multiaddr>().await
    };

    // Discover relay.
    match discover(&mut swarm, relay_address.clone()).await {
        Ok(()) => (),
        Err(err) => {
            let _ = sender.unbounded_send(Message::SwarmCreationError(err));
            // Wait until backend drops stream.
            future::pending::<()>().await
        }
    }

    for node in stream.nodes.0.iter() {
        let _ = swarm.dial(
            relay_address
                .clone()
                .with(Protocol::P2pCircuit)
                .with(Protocol::P2p(node.1.peer_id)),
        );
    }

    let _ = swarm.listen_on(relay_address.clone().with(Protocol::P2pCircuit));

    let (input_sender, input_receiver) = mpsc::channel(100);

    let _ = sender.unbounded_send(Message::SwarmCreated(input_sender));

    let mut input_streams = stream::select(swarm.map(Input::Swarm), input_receiver);

    loop {
        let response = match input_streams.select_next_some().await {
            Input::Swarm(event) => match event {
                libp2p::swarm::SwarmEvent::Behaviour(event) => match event {
                    BehaviourEvent::Identify(_) => None,
                    BehaviourEvent::Ping(_) => None,
                    BehaviourEvent::RelayClient(_) => None,
                    BehaviourEvent::Dcutr(event) => match event.result {
                        Ok(connection_id) => {
                            log::info!(
                                "[swarm] Successfully upgraded connection to {}",
                                event.remote_peer_id
                            );

                            Some(Message::NodeConnected(event.remote_peer_id))
                        }
                        Err(err) => {
                            log::error!("[swarm] Hole punching failure: {err}");
                            None
                        }
                    },
                },
                libp2p::swarm::SwarmEvent::ConnectionEstablished {
                    peer_id,
                    connection_id,
                    endpoint,
                    num_established,
                    concurrent_dial_errors,
                    established_in,
                } => {
                    log::info!(
                        "[swarm] Established relayed connection to {peer_id} via {endpoint:?}."
                    );
                    None
                }
                libp2p::swarm::SwarmEvent::ConnectionClosed {
                    peer_id,
                    connection_id,
                    endpoint,
                    num_established,
                    cause,
                } => {
                    log::info!("[swarm] Closed connection {endpoint:?}");
                    match endpoint {
                        ConnectedPoint::Dialer { address, .. } => {
                            if address == relay_address {
                                None
                            } else {
                                Some(Message::NodeDisconnected(peer_id))
                            }
                        }
                        ConnectedPoint::Listener { .. } => Some(Message::NodeDisconnected(peer_id)),
                    }
                }
                libp2p::swarm::SwarmEvent::IncomingConnection {
                    connection_id,
                    local_addr,
                    send_back_addr,
                } => None,
                libp2p::swarm::SwarmEvent::IncomingConnectionError {
                    connection_id,
                    local_addr,
                    send_back_addr,
                    error,
                    peer_id,
                } => None,
                libp2p::swarm::SwarmEvent::OutgoingConnectionError {
                    connection_id,
                    peer_id,
                    error,
                } => None,
                libp2p::swarm::SwarmEvent::NewListenAddr {
                    listener_id,
                    address,
                } => {
                    log::info!("[swarm] Started listening on {address}");
                    None
                }
                libp2p::swarm::SwarmEvent::ExpiredListenAddr {
                    listener_id,
                    address,
                } => None,
                libp2p::swarm::SwarmEvent::ListenerClosed {
                    listener_id,
                    addresses,
                    reason,
                } => {
                    log::info!("[swarm] Stopped listening on {addresses:?}");
                    None
                }
                libp2p::swarm::SwarmEvent::ListenerError { listener_id, error } => None,
                libp2p::swarm::SwarmEvent::Dialing {
                    peer_id,
                    connection_id,
                } => None,
                libp2p::swarm::SwarmEvent::NewExternalAddrCandidate { address } => None,
                libp2p::swarm::SwarmEvent::ExternalAddrConfirmed { address } => None,
                libp2p::swarm::SwarmEvent::ExternalAddrExpired { address } => None,
                libp2p::swarm::SwarmEvent::NewExternalAddrOfPeer { peer_id, address } => None,
                _ => None,
            },
            Input::Close => None,
        };

        if let Some(response) = response {
            let _ = sender.unbounded_send(response);
        }
    }
}

fn create_swarm(keys: &identity::Keys) -> Result<Swarm, Error> {
    let keypair = Keypair::from(ed25519::Keypair::from(
        ed25519::SecretKey::try_from_bytes(&mut keys.private.as_bytes().to_vec())
            .map_err(|_| Error::KeyDecoding)?,
    ));

    let swarm = SwarmBuilder::with_existing_identity(keypair)
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )
        .expect("expected correct tcp configuration.")
        .with_relay_client(noise::Config::new, yamux::Config::default)
        .expect("expected correct relay client")
        .with_behaviour(|keypair, relay_client| Behaviour::new(keypair, relay_client))
        .expect("expected correct network behavior")
        .build();

    Ok(swarm)
}

#[derive(NetworkBehaviour)]
pub struct Behaviour {
    identify: identify::Behaviour,
    ping: ping::Behaviour,
    relay_client: libp2p::relay::client::Behaviour,
    dcutr: dcutr::Behaviour,
}

impl Behaviour {
    pub fn new(keypair: &Keypair, relay_client: libp2p::relay::client::Behaviour) -> Self {
        Self {
            identify: identify::Behaviour::new(identify::Config::new_with_signed_peer_record(
                "1.0.0".to_string(),
                keypair,
            )),
            ping: ping::Behaviour::default(),
            relay_client,
            dcutr: dcutr::Behaviour::new(keypair.public().to_peer_id()),
        }
    }
}

/// Query the swarm to learn the local nodes public address. The discovery process will also ensure
/// that any freshly started relays will get to know their own public address as well.
async fn discover(swarm: &mut Swarm, relay_addr: Multiaddr) -> Result<(), Error> {
    log::info!("[swarm] Starting relay discovery process on {relay_addr}");

    swarm
        .dial(relay_addr)
        .map_err(|err| Error::DialError(err.to_string()))?;

    let mut learned_observed_addr = false;
    let mut told_relay_observed_addr = false;

    let start = Instant::now();

    loop {
        if start.elapsed() >= Duration::from_secs(2) {
            log::warn!("[swarm] Relay discovery timed out...");
            return Err(Error::TimedOut);
        }

        match swarm.next().await.unwrap() {
            SwarmEvent::NewListenAddr { .. } => {}
            SwarmEvent::Dialing { .. } => {}
            SwarmEvent::ConnectionEstablished { .. } => {}
            SwarmEvent::Behaviour(BehaviourEvent::Ping(_)) => {}
            SwarmEvent::Behaviour(BehaviourEvent::Identify(identify::Event::Sent { .. })) => {
                log::info!("[swarm] Told relay its public address.");
                told_relay_observed_addr = true;
            }
            SwarmEvent::Behaviour(BehaviourEvent::Identify(identify::Event::Received {
                info: identify::Info { observed_addr, .. },
                ..
            })) => {
                log::info!("[swarm] Relay told us our observed address: {observed_addr}");
                learned_observed_addr = true;
            }
            SwarmEvent::OutgoingConnectionError { error, .. } => match error {
                DialError::Transport(error) => {
                    log::error!("[swarm] Encountered transport error during discovery.");
                    return Err(Error::TransportError(
                        error
                            .first()
                            .expect("expected transport error vec to have exactly one entry")
                            .1
                            .to_string(),
                    ));
                }
                _ => panic!("{error}"),
            },
            event => panic!("{event:?}"),
        }

        if learned_observed_addr && told_relay_observed_addr {
            log::info!("[swarm] Relay discovery finished.");
            return Ok(());
        }
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("error decoding private key")]
    KeyDecoding,
    #[error("missing relay address in config")]
    MissingRelayAddress,
    #[error("faulty relay address in config")]
    FaultyRelayAddress,
    #[error("node needs to be relayed")]
    NotRelayedNode,
    #[error("node address not relayed")]
    AddressNotRelayed,
    #[error("transport error: {0}")]
    TransportError(String),
    #[error("dial error: {0}")]
    DialError(String),
    #[error("discovery process timed out")]
    TimedOut,
}
