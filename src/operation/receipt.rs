use serde::Deserialize;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::response::errors::Error;
use crate::types::EthHexFormat;
use crate::types::TransactionHash;
use crate::{
    request::rpc_request::{RpcRequest, ToRpcRequest},
    response::{
        rpc_subscription_success_response::RpcSubscriptionResponse,
        rpc_sync_success_response::{FromRpcSyncSuccessResponse, RpcSyncSuccessResponse},
    },
};
// -----------------------------------------------------------------------------
// Request
// -----------------------------------------------------------------------------
#[derive(Debug)]
pub struct ReceiptRequest {
    pub hash: TransactionHash,
}

impl ToRpcRequest for ReceiptRequest {
    fn to_rpc_request(&self) -> RpcRequest {
        RpcRequest::new(
            "Receipt",
            "eth_getTransactionReceipt",
            vec![self.hash.formatted_hex()],
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
pub struct ReceiptResponse {
    pub receipt: Option<BlockchainReceipt>,
}

#[derive(Debug, Deserialize)]
pub struct BlockchainReceipt {
    #[serde(deserialize_with = "crate::utils::u64_from_hex")]
    pub status: u64,
}

impl BlockchainReceipt {
    pub fn is_confirmed(&self) -> bool {
        self.status == 1
    }
}

impl FromRpcSyncSuccessResponse for ReceiptResponse {
    fn from_rpc_response(
        rpc: RpcSyncSuccessResponse,
        _rx_subscription: UnboundedReceiver<RpcSubscriptionResponse>,
    ) -> Result<Self, Error> {
        match rpc.result {
            // transaction found
            result @ serde_json::Value::Object(_) => {
                match serde_json::from_value::<BlockchainReceipt>(result.clone()) {
                    Ok(receipt) => Ok(ReceiptResponse {
                        receipt: Some(receipt),
                    }),
                    Err(e) => {
                        tracing::error!(reason = ?e, payload = ?result, "failed to parse response from eth_getTransactionReceipt");
                        Err(Error::Parsing(
                            "Failed to parse eth_getTransactionReceipt response".to_string(),
                        ))
                    }
                }
            }
            // transaction not found
            serde_json::Value::Null => Ok(ReceiptResponse { receipt: None }),

            // unexpected format
            _ => {
                tracing::error!(payload = ?rpc.result, "unexpected response for eth_getTransactionReceipt");
                Err(Error::Parsing("Unexpected response format for eth_getTransactionReceipt because result is not an object or null".to_string()))
            }
        }
    }
}
