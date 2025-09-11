use ed25519_dalek::{Signature, Signer, SigningKey};
use futures::Stream;
use iroh::PublicKey;
use serde::{Deserialize, Serialize};

use super::channel::RecvError;

pub type Sender = super::channel::Sender<Message>;
pub type Receiver = super::channel::Receiver<Message>;

#[derive(Debug, Serialize, Deserialize)]
pub enum Message {
    Hello,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SignedMessage {
    pub from: PublicKey,
    pub data: Box<[u8]>,
    pub signature: Signature,
}

impl SignedMessage {
    pub fn verify_and_decode(data: &[u8]) -> Result<(PublicKey, Message), message::Error> {
        let signed_message: SignedMessage = postcard::from_bytes(data)?;
        let key = signed_message.from;

        key.verify(&signed_message.data, &signed_message.signature)?;

        let message = postcard::from_bytes(&signed_message.data)?;

        Ok((key, message))
    }

    pub fn encode_and_sign(
        secret_key: &SigningKey,
        message: &Message,
    ) -> Result<Box<[u8]>, message::Error> {
        let data = postcard::to_stdvec(message)?.into_boxed_slice();
        let signature = secret_key.sign(&data);
        let from = secret_key.verifying_key().into();
        let signed_message = SignedMessage {
            from,
            data,
            signature,
        };

        Ok(postcard::to_stdvec(&signed_message)?.into_boxed_slice())
    }
}

pub mod message {
    #[derive(Debug, thiserror::Error)]
    pub enum Error {
        #[error(transparent)]
        Postcard(#[from] postcard::Error),
        #[error(transparent)]
        Signature(#[from] ed25519_dalek::SignatureError),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    ConnectionError(#[from] iroh::endpoint::ConnectionError),
    #[error(transparent)]
    ConnectError(#[from] iroh::endpoint::ConnectError),
}
