use near_sdk::{env, FunctionError};
use near_sdk::borsh::{BorshSerialize, BorshDeserialize};

#[derive(Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub enum RelayerError {
    Unauthorized,
    InsufficientGasPool,
    InvalidNonce,
    ExpiredTransaction,
    ContractPaused,
    InvalidAccountId,
}

impl FunctionError for RelayerError {
    fn panic(&self) -> ! {
        env::panic_str(&format!("RelayerError: {:?}", self))
    }
}