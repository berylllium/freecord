use std::{
    hash::Hash,
    io::{Read, Write},
    path::PathBuf,
};

use ed25519_dalek::{
    SECRET_KEY_LENGTH, SigningKey, VerifyingKey,
    pkcs8::{
        self,
        spki::der::{pem::LineEnding, zeroize::Zeroizing},
    },
};
use libp2p::PeerId;
use rand::rngs::OsRng;

use crate::environment;

const IDENTITY_DATA_FOLDER_NAME: &str = "identity";
const SECRET_FILE_NAME: &str = "secret.key";
const PUBLIC_FILE_NAME: &str = "public.key";

#[derive(Clone)]
pub struct Identity {
    pub keys: Option<Keys>,
}

impl Identity {
    /// Loads all identities from disk.
    pub fn load() -> Result<Self, Error> {
        let keys = Keys::load()?;

        Ok(Self { keys: Some(keys) })
    }

    pub fn save(&self) -> Result<(), Error> {
        if let Some(keys) = &self.keys {
            keys.save_private_key()?;
            keys.save_public_key()?;
        }

        Ok(())
    }
}

impl Default for Identity {
    fn default() -> Self {
        Self { keys: None }
    }
}

#[derive(Clone)]
pub struct Keys {
    pub private: SigningKey,
    pub public: VerifyingKey,
}

impl Keys {
    /// Generate a new ed25519 keypair.
    pub fn generate() -> Self {
        let mut csprng = OsRng;

        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();

        Self {
            private: signing_key,
            public: verifying_key,
        }
    }

    pub fn load() -> Result<Self, Error> {
        let mut buffer = [0; SECRET_KEY_LENGTH];

        if !Self::private_path().exists() {
            return Err(Error::NoPrivateKeyOnDisk);
        }

        std::fs::OpenOptions::new()
            .read(true)
            .open(Self::private_path())?
            .read(&mut buffer)?;

        let signing_key = SigningKey::from_bytes(&buffer);
        let verifying_key = signing_key.verifying_key();

        Ok(Self {
            private: signing_key,
            public: verifying_key,
        })
    }

    pub fn save_private_key(&self) -> Result<(), Error> {
        std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(Self::private_path())?
            .write(self.private.as_bytes())?;

        Ok(())
    }

    pub fn save_public_key(&self) -> Result<(), Error> {
        std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(Self::public_path())?
            .write(self.public.as_bytes())?;

        Ok(())
    }

    /// Deletes the keypair from disk, does nothing if keys don't exist.
    pub fn delete_keypair(&self) {
        if Self::private_path().exists() {
            let _ = std::fs::remove_file(Self::private_path());
        }

        if Self::public_path().exists() {
            let _ = std::fs::remove_file(Self::public_path());
        }
    }

    pub fn private_key_pem(&self) -> Result<Zeroizing<String>, Error> {
        use ed25519_dalek::pkcs8::EncodePrivateKey;

        Ok(self.private.to_pkcs8_pem(LineEnding::default())?)
    }

    pub fn public_key_pem(&self) -> Result<String, Error> {
        use ed25519_dalek::pkcs8::EncodePublicKey;

        Ok(self.public.to_public_key_pem(LineEnding::default())?)
    }

    pub fn peer_id(&self) -> PeerId {
        use libp2p::identity;

        let public_key = identity::PublicKey::from(
            identity::ed25519::PublicKey::try_from_bytes(self.public.as_bytes())
                .expect("expected public key to be valid"),
        );

        PeerId::from_public_key(&public_key)
    }

    pub fn dir() -> PathBuf {
        let dir = environment::data_dir().join(IDENTITY_DATA_FOLDER_NAME);

        if !dir.exists() {
            std::fs::create_dir_all(dir.as_path())
                .expect("expected permissions to create identity data folder");
        }

        dir
    }

    pub fn private_path() -> PathBuf {
        Self::dir().join(SECRET_FILE_NAME)
    }

    pub fn public_path() -> PathBuf {
        Self::dir().join(PUBLIC_FILE_NAME)
    }
}

impl Hash for Keys {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.public.hash(state);
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("No keypair associated with this identity")]
    NoKeypair,
    #[error("No private key was found on disk")]
    NoPrivateKeyOnDisk,
    #[error(transparent)]
    EncodePrivateKeyError(#[from] pkcs8::Error),
    #[error(transparent)]
    EncodePublicKeyError(#[from] pkcs8::spki::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
