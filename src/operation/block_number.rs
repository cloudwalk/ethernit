use tokio::sync::mpsc::UnboundedReceiver;

use crate::{
    request::rpc_request::{RpcRequest, ToRpcRequest},
    response::{
        errors::Error,
        rpc_subscription_success_response::RpcSubscriptionResponse,
        rpc_sync_success_response::{FromRpcSyncSuccessResponse, RpcSyncSuccessResponse},
    },
    types::BlockNumber,
};

// -----------------------------------------------------------------------------
// Request
// -----------------------------------------------------------------------------
#[derive(Debug)]
pub struct BlockNumberRequest;

impl ToRpcRequest for BlockNumberRequest {
    fn to_rpc_request(&self) -> RpcRequest {
        RpcRequest::new("BlockNumber", "eth_blockNumber", vec![] as Vec<String>)
    }

    fn is_subscription(&self) -> bool {
        false
    }
}

// -----------------------------------------------------------------------------
// Response
// -----------------------------------------------------------------------------
#[derive(Debug)]
pub struct BlockNumberResponse {
    pub block_number: BlockNumber,
}

impl FromRpcSyncSuccessResponse for BlockNumberResponse {
    fn from_rpc_response(
        rpc: RpcSyncSuccessResponse,
        _rx_subscription: UnboundedReceiver<RpcSubscriptionResponse>,
    ) -> Result<Self, Error> {
        let mut result = rpc.result_as_string()?;
        result = result.trim_start_matches("0x").into();

        let block_number = BlockNumber::from_str_radix(&result, 16).map_err(|_| {
            Error::Parsing(format!(
                "Failed to parse blockchain RPC block number result | rpc={result}"
            ))
        })?;
        Ok(BlockNumberResponse { block_number })
    }
}
