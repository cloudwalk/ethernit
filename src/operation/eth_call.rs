use crate::request::{
    rpc_request::{RpcRequest, ToRpcRequest},
    rpc_request_contract_call::RpcRequestContractCall,
};

// -----------------------------------------------------------------------------
// Request
// -----------------------------------------------------------------------------
#[derive(Debug)]
pub struct EthCallRequest {
    pub id_prefix: String,
    pub payload: RpcRequestContractCall,
}

impl ToRpcRequest for EthCallRequest {
    fn to_rpc_request(&self) -> RpcRequest {
        RpcRequest::new(
            &self.id_prefix,
            "eth_call",
            vec![serde_json::to_value(&self.payload).unwrap()],
        )
    }

    fn is_subscription(&self) -> bool {
        false
    }
}
