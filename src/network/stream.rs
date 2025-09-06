use futures::{StreamExt, channel::mpsc, stream};
use iced::advanced::{
    graphics::futures::BoxStream,
    subscription::{EventStream, Hasher, Recipe},
};

use crate::identity::Keys;

#[derive(Clone)]
pub struct Stream {
    pub keys: Keys,
}

impl Recipe for Stream {
    type Output = super::Message;

    fn hash(&self, state: &mut Hasher) {
        use std::hash::Hash;

        self.keys.hash(state);
    }

    fn stream(self: Box<Self>, _input: EventStream) -> BoxStream<Self::Output> {
        let (backend_sender, backend_receiver) = mpsc::unbounded();

        let runner = stream::once(async { tokio::spawn(super::run(self, backend_sender)).await })
            .map(|_| unreachable!());

        stream::select(backend_receiver, runner).boxed()
    }
}
