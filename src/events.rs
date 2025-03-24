use near_sdk::{near, AccountId, NearToken, PublicKey};

#[near(event_json(standard = "nep297"))]
pub enum RelayerEvent {
    #[event_version("1.0.0")]
    MetaTransactionRelayed { sender_id: AccountId, nonce: u64 },
    #[event_version("1.0.0")]
    AccountSponsored { account_id: AccountId, public_key: PublicKey, is_implicit: bool },
    #[event_version("1.0.0")]
    GasPoolDeposited { amount: NearToken, depositor: AccountId },
    #[event_version("1.0.0")]
    FunctionCallKeyAdded { account_id: AccountId, public_key: PublicKey, receiver_id: AccountId },
    #[event_version("1.0.0")]
    FailedTransactionsCleared { count: u32 },
    #[event_version("1.0.0")]
    FailedTransactionsRetried { count: u32 },
}