use futures::stream;
use iced::{
    Subscription,
    advanced::{graphics::futures::BoxStream, subscription},
};
use libp2p::{
    Swarm, SwarmBuilder, identify,
    identity::Keypair,
    noise,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux,
};

#[derive(Debug)]
pub enum Message {
    Identify(identify::Event),
}

pub struct Event {}

#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "Message")]
struct Behaviour {
    identify: identify::Behaviour,
}

pub fn listen() -> Subscription<Message> {
    use futures::stream::StreamExt;

    struct Listener;

    impl subscription::Recipe for Listener {
        type Output = Message;

        fn hash(&self, state: &mut subscription::Hasher) {
            use std::hash::Hash;
            struct Marker;
            std::any::TypeId::of::<Marker>().hash(state)
        }

        fn stream(self: Box<Self>, _input: subscription::EventStream) -> BoxStream<Self::Output> {
            println!("New swarm.");

            let mut swarm = SwarmBuilder::with_new_identity()
                .with_tokio()
                .with_tcp(
                    tcp::Config::default(),
                    noise::Config::new,
                    yamux::Config::default,
                )
                .expect("expected correct tcp configuration.")
                .with_behaviour(|keypair| Behaviour::new(keypair))
                .expect("expected correct network behavior")
                .build();

            swarm
                .listen_on("/ip4/127.0.0.1/tcp/5010".parse().unwrap())
                .unwrap();

            stream::unfold(swarm, |mut swarm| async move {
                match swarm.select_next_some().await {
                    SwarmEvent::Behaviour(event) => Some((Some(event), swarm)),
                    SwarmEvent::NewListenAddr { address, .. } => {
                        println!("{address:?}");
                        Some((None, swarm))
                    }
                    _ => Some((None, swarm)),
                }
            })
            .filter_map(|value| async move { value })
            .boxed()
        }
    }

    subscription::from_recipe(Listener)
}

impl Behaviour {
    pub fn new(keypair: &Keypair) -> Self {
        Self {
            identify: identify::Behaviour::new(identify::Config::new_with_signed_peer_record(
                "1.0.0".to_string(),
                keypair,
            )),
        }
    }
}

impl From<identify::Event> for Message {
    fn from(value: identify::Event) -> Self {
        Self::Identify(value)
    }
}
