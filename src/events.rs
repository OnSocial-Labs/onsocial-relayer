use near_sdk::{AccountId, near};

#[near(event_json(standard = "nep297"))]
#[derive(Debug)]
pub enum RelayerEvent {
    #[event_version("1.0.0")]
    AuthAdded { auth_account: AccountId },
    #[event_version("1.0.0")]
    AuthRemoved { auth_account: AccountId },
    #[event_version("1.0.0")]
    AdminAdded { admin_account: AccountId },
    #[event_version("1.0.0")]
    AdminRemoved { admin_account: AccountId },
    #[event_version("1.0.0")]
    SponsorAmountUpdated { new_amount: u128 },
    #[event_version("1.0.0")]
    OffloadRecipientUpdated { new_recipient: AccountId },
    #[event_version("1.0.0")]
    MaxGasPoolUpdated { new_max: u128 },
    #[event_version("1.0.0")]
    MinGasPoolUpdated { new_min: u128 },
    #[event_version("1.0.0")]
    LowGasPool { remaining: u128 },
    #[event_version("1.0.0")]
    ChainMpcMappingAdded { chain: String, mpc_contract: AccountId },
    #[event_version("1.0.0")]
    ChainMpcMappingRemoved { chain: String },
    #[event_version("1.0.0")]
    ChunkSizeUpdated { new_size: usize },
    #[event_version("1.0.0")]
    CrossChainSignatureResult { chain: String, request_id: u64, result: Vec<u8> },
    #[event_version("1.0.0")]
    BridgeResult { sender_id: AccountId, action_type: String, result: Vec<u8> }, // NEW: For bridge outcomes
}