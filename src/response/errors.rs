use thiserror::Error;

use super::rpc_sync_error_response::RpcSyncErrorResponseError;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    Parsing(String),
    #[error("{0}")]
    SyncResponse(RpcSyncErrorResponseError),
}
