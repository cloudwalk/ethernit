use serde::Deserialize;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::response::errors::Error;
use crate::types::ChainId;
use crate::types::ContractAddress;
use crate::types::EthHexFormat;
use crate::types::Nonce;
use crate::types::TransactionHash;
use crate::types::WalletAddress;
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
pub struct TransactionRequest {
    pub hash: TransactionHash,
}

impl ToRpcRequest for TransactionRequest {
    fn to_rpc_request(&self) -> RpcRequest {
        RpcRequest::new(
            "Transaction",
            "eth_getTransactionByHash",
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
pub struct TransactionResponse {
    pub transaction: Option<BlockchainTransaction>,
}

#[derive(Debug, Deserialize)]
pub struct BlockchainTransaction {
    #[serde(
        deserialize_with = "crate::utils::u64_from_hex",
        rename(deserialize = "chainId")
    )]
    pub chain_id: ChainId,
    pub nonce: Nonce,
    pub from: WalletAddress,
    pub to: ContractAddress,
    #[serde(deserialize_with = "crate::utils::bytes_from_hex")]
    pub input: Vec<u8>,
}

impl FromRpcSyncSuccessResponse for TransactionResponse {
    fn from_rpc_response(
        rpc: RpcSyncSuccessResponse,
        _rx_subscription: UnboundedReceiver<RpcSubscriptionResponse>,
    ) -> Result<Self, Error> {
        match rpc.result {
            // transaction found
            result @ serde_json::Value::Object(_) => {
                match serde_json::from_value::<BlockchainTransaction>(result.clone()) {
                    Ok(transaction) => Ok(TransactionResponse {
                        transaction: Some(transaction),
                    }),
                    Err(e) => {
                        tracing::error!(reason = ?e, payload = ?result, "failed to parse response from eth_getTransactionByHash");
                        Err(Error::Parsing(
                            "Failed to parse eth_getTransactionByHash response".to_string(),
                        ))
                    }
                }
            }
            // transaction not found
            serde_json::Value::Null => Ok(TransactionResponse { transaction: None }),

            // unexpected format
            _ => {
                tracing::error!(payload = ?rpc.result, "unexpected response for eth_getTransactionByHash");
                Err(Error::Parsing("Unexpected response format for eth_getTransactionByHash because result is not an object or null".to_string()))
            }
        }
    }
}
