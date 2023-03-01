use ethabi::Token;
use serde::Serialize;

use crate::types::{Amount, ContractAddress, WalletAddress};

#[derive(Debug, Serialize, Clone)]
pub struct RpcRequestContractCall {
    from: WalletAddress,
    to: ContractAddress,
    data: String,
    gas: Amount,
    #[serde(rename(serialize = "gasPrice"))]
    gas_price: Amount,
}

impl RpcRequestContractCall {
    pub fn new(
        from: WalletAddress,
        contract_address: ContractAddress,
        function_hash: [u8; 4],
        function_params: Vec<Token>,
        gas: Amount,
        gas_price: Amount,
    ) -> Self {
        let function_params_bytes = ethabi::encode(&function_params);
        let data = function_hash
            .into_iter()
            .chain(function_params_bytes.into_iter())
            .collect::<Vec<u8>>();

        Self {
            from,
            to: contract_address,
            data: format!("0x{}", hex::encode(data)),
            gas,
            gas_price,
        }
    }
}
