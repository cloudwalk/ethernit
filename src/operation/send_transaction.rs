use std::str::FromStr;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::{
    request::rpc_request::{RpcRequest, ToRpcRequest},
    response::{
        errors::Error,
        rpc_subscription_success_response::RpcSubscriptionResponse,
        rpc_sync_success_response::{FromRpcSyncSuccessResponse, RpcSyncSuccessResponse},
    },
    types::{SignedData, TransactionHash},
};

// -----------------------------------------------------------------------------
// Request
// -----------------------------------------------------------------------------
#[derive(Debug)]
pub struct SendTransactionRequest {
    pub signed_data: SignedData,
}

impl ToRpcRequest for SendTransactionRequest {
    fn to_rpc_request(&self) -> RpcRequest {
        RpcRequest::new(
            "SendTransaction",
            "eth_sendRawTransaction",
            vec![self.signed_data.clone()],
        )
    }

    fn is_subscription(&self) -> bool {
        false
    }
}

// -----------------------------------------------------------------------------
// Response
// -----------------------------------------------------------------------------
#[derive(Debug)]
pub struct SendTransactionResponse {
    pub hash: TransactionHash,
}

impl FromRpcSyncSuccessResponse for SendTransactionResponse {
    fn from_rpc_response(
        rpc: RpcSyncSuccessResponse,
        _rx_subscription: UnboundedReceiver<RpcSubscriptionResponse>,
    ) -> Result<Self, Error> {
        let result = rpc.result_as_string()?;
        let hash = TransactionHash::from_str(&result).map_err(|_| {
            Error::Parsing(format!(
                "Failed to parse blockchain RPC transaction hash result | rpc={result}"
            ))
        })?;
        Ok(SendTransactionResponse { hash })
    }
}
