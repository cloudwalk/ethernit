use serde::Deserialize;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::request::RequestId;

use super::{errors::Error, rpc_subscription_success_response::RpcSubscriptionResponse};

/// Converts RPC success responses to struct.
pub trait FromRpcSyncSuccessResponse: Sized + core::fmt::Debug {
    /// Converts an Ethereum RPC success response to Self
    fn from_rpc_response(
        rpc: RpcSyncSuccessResponse,
        rx_subscription: UnboundedReceiver<RpcSubscriptionResponse>,
    ) -> Result<Self, Error>;
}

#[derive(Debug, Clone, Deserialize)]
pub struct RpcSyncSuccessResponse {
    pub id: RequestId,
    pub result: serde_json::Value,
}

impl RpcSyncSuccessResponse {
    pub fn result_as_string(&self) -> Result<String, Error> {
        match self.result.as_str().map(|s| s.to_owned()) {
            Some(res) => Ok(res),
            None => Err(Error::Parsing(
                "Failed to convert blockchain RPC result field as string".to_string(),
            )),
        }
    }

    pub fn result_as_bool(&self) -> Result<bool, Error> {
        match self.result.as_bool() {
            Some(res) => Ok(res),
            None => Err(Error::Parsing(
                "Failed to convert blockchain RPC result field as bool".to_string(),
            )),
        }
    }
}
