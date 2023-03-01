use thiserror::Error;

use crate::response::errors::Error as RpcResponseError;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to open connection with blockchain. Error = '{0}'")]
    Connection(String),
    #[error("Failed to send blockchain request to internal equest manager")]
    RequestManagerSend,
    #[error("Failed to receive message at internal request manager")]
    RequestManagerReceive,
    #[error("Received error response from blockchain when subscribing to event. Payload = '{0}'")]
    Subscription(String),
    #[error("Timeout at request")]
    Timeout,
    #[error(transparent)]
    RpcResponse(#[from] RpcResponseError),
    #[error("Failed to create block number placeholder struct")]
    BlockNumber,
}
