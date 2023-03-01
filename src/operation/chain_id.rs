use tokio::sync::mpsc::UnboundedReceiver;

use crate::{
    request::rpc_request::{RpcRequest, ToRpcRequest},
    response::{
        errors::Error,
        rpc_subscription_success_response::RpcSubscriptionResponse,
        rpc_sync_success_response::{FromRpcSyncSuccessResponse, RpcSyncSuccessResponse},
    },
    types::ChainId,
};
// -----------------------------------------------------------------------------
// Request
// -----------------------------------------------------------------------------
#[derive(Debug)]
pub struct ChainIdRequest;

impl ToRpcRequest for ChainIdRequest {
    fn to_rpc_request(&self) -> RpcRequest {
        RpcRequest::new("ChainId", "eth_chainId", vec![] as Vec<String>)
    }

    fn is_subscription(&self) -> bool {
        false
    }
}

// -----------------------------------------------------------------------------
// Response
// -----------------------------------------------------------------------------
#[derive(Debug)]
pub struct ChainIdResponse {
    pub chain_id: ChainId,
}

impl FromRpcSyncSuccessResponse for ChainIdResponse {
    fn from_rpc_response(
        rpc: RpcSyncSuccessResponse,
        _rx_subscription: UnboundedReceiver<RpcSubscriptionResponse>,
    ) -> Result<Self, Error> {
        let mut result = rpc.result_as_string()?;
        result = result.trim_start_matches("0x").into();

        let chain_id = ChainId::from_str_radix(&result, 16).map_err(|_| {
            Error::Parsing(format!(
                "Failed to parse blockchain RPC chain id result | rpc={result}"
            ))
        })?;
        Ok(ChainIdResponse { chain_id })
    }
}
