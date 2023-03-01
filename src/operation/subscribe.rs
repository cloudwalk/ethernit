use tokio::sync::mpsc::UnboundedReceiver;

use crate::response::{
    errors::Error,
    rpc_subscription_success_response::RpcSubscriptionResponse,
    rpc_sync_success_response::{FromRpcSyncSuccessResponse, RpcSyncSuccessResponse},
};

// -----------------------------------------------------------------------------
// Response
// -----------------------------------------------------------------------------
#[derive(Debug)]
pub struct SubscribeResponse {
    pub id: String,
    pub rx_subscription: UnboundedReceiver<RpcSubscriptionResponse>,
}

impl FromRpcSyncSuccessResponse for SubscribeResponse {
    fn from_rpc_response(
        rpc: RpcSyncSuccessResponse,
        rx_subscription: UnboundedReceiver<RpcSubscriptionResponse>,
    ) -> Result<Self, Error> {
        Ok(SubscribeResponse {
            id: rpc.result_as_string()?,
            rx_subscription,
        })
    }
}
