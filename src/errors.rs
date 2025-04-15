use near_sdk::{env, FunctionError};
use near_sdk::borsh::{BorshSerialize, BorshDeserialize};

#[derive(Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub enum RelayerError {
    Unauthorized,
    InsufficientBalance,
    InvalidNonce,
    ExpiredTransaction,
    ContractPaused,
    InvalidAccountId,
    AmountTooLow,
    AmountTooHigh,
    LastAdmin,
    AlreadyExists,
    CannotSelfRemove,
    NotFound,
    InvalidSignature,
    InvalidRequestId,
    InvalidChainId,
    AccountTaken,
    RateLimitHit,
    KeyExpired,
    InsufficientDeposit,
}

impl FunctionError for RelayerError {
    fn panic(&self) -> ! {
        env::panic_str(&format!("RelayerError: {:?}", self))
    }
}
