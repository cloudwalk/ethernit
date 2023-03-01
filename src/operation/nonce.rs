use std::str::FromStr;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::{
    request::rpc_request::{RpcRequest, ToRpcRequest},
    response::{
        errors::Error,
        rpc_subscription_success_response::RpcSubscriptionResponse,
        rpc_sync_success_response::{FromRpcSyncSuccessResponse, RpcSyncSuccessResponse},
    },
    types::{EthHexFormat, Nonce, WalletAddress},
};

// -----------------------------------------------------------------------------
// Request
// -----------------------------------------------------------------------------
#[derive(Debug)]
pub struct NonceRequest {
    pub wallet: WalletAddress,
}

impl ToRpcRequest for NonceRequest {
    fn to_rpc_request(&self) -> RpcRequest {
        RpcRequest::new(
            "Nonce",
            "eth_getTransactionCount",
            vec![self.wallet.formatted_hex(), "pending".to_string()],
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
pub struct NonceResponse {
    pub nonce: Nonce,
}

impl FromRpcSyncSuccessResponse for NonceResponse {
    fn from_rpc_response(
        rpc: RpcSyncSuccessResponse,
        _rx_subscription: UnboundedReceiver<RpcSubscriptionResponse>,
    ) -> Result<Self, Error> {
        let result = rpc.result_as_string()?;
        let nonce = Nonce::from_str(&result).map_err(|_| {
            Error::Parsing(format!(
                "Failed to parse blockchain RPC nonce result | payload={result}"
            ))
        })?;
        Ok(Self { nonce })
    }
}
