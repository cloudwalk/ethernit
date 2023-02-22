use serde::Deserialize;

use crate::{
    request::rpc_request::{RpcRequest, ToRpcRequest},
    types::{ContractAddress, TransactionHash},
};
// -----------------------------------------------------------------------------
// Request
// -----------------------------------------------------------------------------
#[derive(Debug)]
pub struct SubscribeLogsRequest {
    pub contract: ContractAddress,
    pub topics: Vec<serde_json::Value>,
}

impl ToRpcRequest for SubscribeLogsRequest {
    fn to_rpc_request(&self) -> RpcRequest {
        let params = {
            let mut filter = serde_json::Map::default();
            filter.insert(
                "address".into(),
                serde_json::to_value(self.contract).unwrap(),
            );
            let topics = serde_json::Value::Array(self.topics.clone());
            filter.insert("topics".into(), topics);
            vec![
                serde_json::Value::String("logs".into()),
                serde_json::Value::Object(filter),
            ]
        };
        RpcRequest::new("SubscribeLogs", "eth_subscribe", params)
    }

    fn is_subscription(&self) -> bool {
        true
    }
}

// -----------------------------------------------------------------------------
// Response
// -----------------------------------------------------------------------------
#[derive(Debug, Deserialize)]
pub struct SubscribeLogsEvent {
    #[serde(rename(deserialize = "transactionHash"))]
    pub transaction_hash: TransactionHash,
    #[serde(
        rename(deserialize = "logIndex"),
        deserialize_with = "crate::utils::u64_from_hex"
    )]
    pub log_index: u64,
    pub removed: bool,
    pub data: String,
}
