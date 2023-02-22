use serde::Deserialize;

use crate::{
    request::rpc_request::{RpcRequest, ToRpcRequest},
    types::BlockNumber,
};
// -----------------------------------------------------------------------------
// Request
// -----------------------------------------------------------------------------
#[derive(Debug)]
pub struct SubscribeNewHeadsRequest;

impl ToRpcRequest for SubscribeNewHeadsRequest {
    fn to_rpc_request(&self) -> RpcRequest {
        RpcRequest::new(
            "SubscribeNewHeads",
            "eth_subscribe",
            vec![serde_json::Value::String("newHeads".into())],
        )
    }

    fn is_subscription(&self) -> bool {
        true
    }
}

// -----------------------------------------------------------------------------
// Response
// -----------------------------------------------------------------------------
#[derive(Debug, Deserialize)]
pub struct SubscribeNewHeadsEvent {
    #[serde(
        rename(deserialize = "number"),
        deserialize_with = "crate::utils::i64_from_hex"
    )]
    pub block_number: BlockNumber,
}
