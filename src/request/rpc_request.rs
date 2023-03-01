use super::RequestId;
use nanoid::nanoid;
use serde::Serialize;

/// Converts struct to RPC requests to be sent to Ethereum RPC.
pub trait ToRpcRequest: Send + Sync + core::fmt::Debug {
    /// Converts Self to an Ethereum RPC request
    fn to_rpc_request(&self) -> RpcRequest;

    /// Indicates that Self is a request that will create an Ethereum subscription
    fn is_subscription(&self) -> bool;
}

#[derive(Debug, Clone, Serialize)]
pub struct RpcRequest {
    pub jsonrpc: &'static str,
    pub id: RequestId,
    pub method: String,
    pub params: serde_json::Value,
}

impl RpcRequest {
    pub fn new(id_prefix: &str, method: &str, params: Vec<impl Serialize>) -> Self {
        Self {
            jsonrpc: "2.0",
            id: format!("{}__{}", id_prefix, nanoid!()),
            method: method.into(),
            params: serde_json::to_value(params).unwrap(),
        }
    }
}
