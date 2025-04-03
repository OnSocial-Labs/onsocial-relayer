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
}