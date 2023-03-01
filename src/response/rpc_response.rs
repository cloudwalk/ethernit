use eyre::WrapErr;
use serde::Deserialize;

use crate::request::RequestId;

use super::{
    rpc_subscription_success_response::RpcSubscriptionResponse,
    rpc_sync_error_response::{RpcSyncErrorResponse, RpcSyncErrorResponseError},
    rpc_sync_success_response::RpcSyncSuccessResponse,
};

#[derive(Debug)]
pub enum RpcResponse {
    Sync(RpcSyncResponse),
    Subscription(RpcSubscriptionResponse),
}

impl RpcResponse {
    pub fn parse(payload: String) -> eyre::Result<RpcResponse> {
        let json: Result<RpcResponseForMatch, serde_json::Error> = serde_json::from_str(&payload);

        // order of matches is important
        match json {
            // sync request error
            Ok(ref rpc @ RpcResponseForMatch { error: Some(_), .. }) => {
                serde_json::from_str::<RpcSyncErrorResponse>(&payload)
                    .map(|it| RpcResponse::Sync(RpcSyncResponse::Error(it)))
                    .wrap_err_with(|| {format!("Unexpected message format from blockchain for response error | rpc={rpc:?}")})
            }

            // async subscription message
            Ok(
                ref rpc @ RpcResponseForMatch {
                    method: Some(ref m),
                    ..
                },
            ) if m.as_str() == "eth_subscription" => {
                serde_json::from_str::<RpcSubscriptionResponse>(&payload)
                    .map(RpcResponse::Subscription)
                    .wrap_err_with(|| {format!("Unexpected message format from blockchain for subscription | rpc={rpc:?}")})
            }

            // sync request result
            Ok(rpc) => serde_json::from_str::<RpcSyncSuccessResponse>(&payload)
                .map(|it| RpcResponse::Sync(RpcSyncResponse::Success(it)))
                .wrap_err_with(|| {format!("Unexpected message format from blockchain for response success | rpc={rpc:?}")}),

            // error message
            Err(e) => Err(e).wrap_err_with(|| {
                format!("Unexpected message format from blockchain | rpc={payload}")
            }),
        }
    }
}

// -----------------------------------------------------------------------------
// Sync Response
// -----------------------------------------------------------------------------
#[derive(Debug)]
pub enum RpcSyncResponse {
    Success(RpcSyncSuccessResponse),
    Error(RpcSyncErrorResponse),
}

impl RpcSyncResponse {
    pub fn id(&self) -> &RequestId {
        match self {
            RpcSyncResponse::Success(res) => &res.id,
            RpcSyncResponse::Error(res) => &res.id,
        }
    }
}

// -----------------------------------------------------------------------------
// Internal
// -----------------------------------------------------------------------------
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct RpcResponseForMatch {
    pub result: Option<serde_json::Value>,
    pub error: Option<RpcSyncErrorResponseError>,
    pub method: Option<String>,
}
