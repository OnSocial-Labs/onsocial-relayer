use near_sdk::env;

#[derive(Debug, PartialEq)]
pub enum RelayerError {
    InsufficientGasPool,
    InvalidNonce,
    NotWhitelisted,
    InvalidSignature,
    NoActions,
    InvalidFTTransfer,
    InsufficientDeposit,
    InsufficientBalance,
    AccountExists,
    Unauthorized,
    InvalidSponsorAmount,
    InvalidKeyAction,
    InvalidAccountId,
    ExpiredTransaction,
    InvalidGasConfig,
    NoFailedTransactions,
}

impl near_sdk::FunctionError for RelayerError {
    fn panic(&self) -> ! {
        env::panic_str(&format!("RelayerError: {:?}", self))
    }
}