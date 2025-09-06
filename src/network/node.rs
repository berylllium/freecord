pub struct Node {}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    ConnectionError(#[from] iroh::endpoint::ConnectionError),
    #[error(transparent)]
    ConnectError(#[from] iroh::endpoint::ConnectError),
}
