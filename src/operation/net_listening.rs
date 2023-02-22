use tokio::sync::mpsc::UnboundedReceiver;

use crate::{
    request::rpc_request::{RpcRequest, ToRpcRequest},
    response::{
        errors::Error,
        rpc_subscription_success_response::RpcSubscriptionResponse,
        rpc_sync_success_response::{FromRpcSyncSuccessResponse, RpcSyncSuccessResponse},
    },
};
// -----------------------------------------------------------------------------
// Request
// -----------------------------------------------------------------------------
#[derive(Debug)]
pub struct NetListeningRequest;

impl ToRpcRequest for NetListeningRequest {
    fn to_rpc_request(&self) -> RpcRequest {
        RpcRequest::new("NetListening", "net_listening", vec![] as Vec<String>)
    }

    fn is_subscription(&self) -> bool {
        false
    }
}

// -----------------------------------------------------------------------------
// Response
// -----------------------------------------------------------------------------
#[derive(Debug)]
pub struct NetListeningResponse {
    pub is_listening: bool,
}

impl FromRpcSyncSuccessResponse for NetListeningResponse {
    fn from_rpc_response(
        rpc: RpcSyncSuccessResponse,
        _rx_subscription: UnboundedReceiver<RpcSubscriptionResponse>,
    ) -> Result<Self, Error> {
        Ok(NetListeningResponse {
            is_listening: rpc.result_as_bool()?,
        })
    }
}
