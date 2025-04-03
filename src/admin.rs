use near_sdk::env;
use near_sdk::{AccountId, PublicKey};
use crate::state::Relayer;
use crate::errors::RelayerError;
use crate::events::RelayerEvent;

pub fn add_auth_account(relayer: &mut Relayer, auth_account: AccountId, auth_public_key: PublicKey) -> Result<(), RelayerError> {
    let caller = env::predecessor_account_id();
    if !relayer.is_admin(&caller) {
        return Err(RelayerError::Unauthorized);
    }
    relayer.auth_accounts.insert(auth_account.clone(), auth_public_key);
    RelayerEvent::AuthAdded { auth_account }.emit();
    Ok(())
}

pub fn remove_auth_account(relayer: &mut Relayer, auth_account: AccountId) -> Result<(), RelayerError> {
    let caller = env::predecessor_account_id();
    if !relayer.is_admin(&caller) {
        return Err(RelayerError::Unauthorized);
    }
    relayer.auth_accounts.remove(&auth_account);
    RelayerEvent::AuthRemoved { auth_account }.emit();
    Ok(())
}

pub fn set_offload_recipient(relayer: &mut Relayer, new_recipient: AccountId) -> Result<(), RelayerError> {
    let caller = env::predecessor_account_id();
    if !relayer.is_admin(&caller) {
        return Err(RelayerError::Unauthorized);
    }
    relayer.offload_recipient = new_recipient.clone();
    RelayerEvent::OffloadRecipientUpdated { new_recipient }.emit();
    Ok(())
}

pub fn add_admin(relayer: &mut Relayer, new_admin: AccountId) -> Result<(), RelayerError> {
    let caller = env::predecessor_account_id();
    if !relayer.is_admin(&caller) {
        return Err(RelayerError::Unauthorized);
    }
    if !relayer.admins.contains(&new_admin) {
        relayer.admins.push(new_admin.clone());
        RelayerEvent::AdminAdded { admin_account: new_admin }.emit();
    }
    Ok(())
}

pub fn remove_admin(relayer: &mut Relayer, admin_to_remove: AccountId) -> Result<(), RelayerError> {
    let caller = env::predecessor_account_id();
    if !relayer.is_admin(&caller) {
        return Err(RelayerError::Unauthorized);
    }
    // Prevent removing the last admin
    if relayer.admins.len() <= 1 {
        return Err(RelayerError::LastAdmin);
    }
    if let Some(index) = relayer.admins.iter().position(|admin| admin == &admin_to_remove) {
        relayer.admins.swap_remove(index);
        RelayerEvent::AdminRemoved { admin_account: admin_to_remove }.emit();
    }
    Ok(())
}

pub fn set_sponsor_amount(relayer: &mut Relayer, new_amount: u128) -> Result<(), RelayerError> {
    let caller = env::predecessor_account_id();
    if !relayer.is_admin(&caller) {
        return Err(RelayerError::Unauthorized);
    }
    // Optional: Add a minimum threshold to prevent setting it too low
    if new_amount < 10_000_000_000_000_000_000_000 { // e.g., 0.01 NEAR
        return Err(RelayerError::AmountTooLow);
    }
    relayer.sponsor_amount = new_amount;
    RelayerEvent::SponsorAmountUpdated { new_amount }.emit();
    Ok(())
}