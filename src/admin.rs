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
    relayer.offload_recipient = new_recipient;
    Ok(())
}