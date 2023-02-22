use ethereum_types::{H160, H256, U128, U256};

// -----------------------------------------------------------------------------
// Friendly type names
// -----------------------------------------------------------------------------
pub type Amount = U256;
pub type BlockNumber = i64;
pub type ChainId = u64;
pub type ContractAddress = H160;
pub type Nonce = U128;
pub type SignedData = String;
pub type TransactionHash = H256;
pub type WalletAddress = H160;

pub trait EthHexFormat {
    fn formatted_hex(&self) -> String;
}
pub trait EthDecFormat {
    fn formatted_dec(&self) -> String;
}

// -----------------------------------------------------------------------------
// H160
// -----------------------------------------------------------------------------
impl EthHexFormat for H160 {
    fn formatted_hex(&self) -> String {
        format!("{self:#x}")
    }
}

// -----------------------------------------------------------------------------
// H256
// -----------------------------------------------------------------------------
impl EthHexFormat for H256 {
    fn formatted_hex(&self) -> String {
        format!("{self:#x}")
    }
}

// -----------------------------------------------------------------------------
// U128
// -----------------------------------------------------------------------------
impl EthDecFormat for U128 {
    fn formatted_dec(&self) -> String {
        self.as_u128().to_string()
    }
}

// -----------------------------------------------------------------------------
// U256
// -----------------------------------------------------------------------------
impl EthDecFormat for U256 {
    fn formatted_dec(&self) -> String {
        self.to_string()
    }
}

impl EthHexFormat for U256 {
    fn formatted_hex(&self) -> String {
        format!("{self:#x}")
    }
}
