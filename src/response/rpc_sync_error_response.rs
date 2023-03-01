use serde::Deserialize;
use std::fmt::{Display, Formatter};
use thiserror::Error;

use crate::request::RequestId;

#[derive(Debug, Deserialize)]
pub struct RpcSyncErrorResponse {
    pub id: RequestId,
    pub jsonrpc: String,
    pub error: RpcSyncErrorResponseError,
}

impl Display for RpcSyncErrorResponseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.code, self.message)
    }
}

#[derive(Debug, Deserialize, Error)]
pub struct RpcSyncErrorResponseError {
    code: i64,
    message: String,
}
