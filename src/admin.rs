use near_sdk::env;
use near_sdk::{AccountId, PublicKey, Gas};
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
    if let Some(index) = relayer.admins.iter().position(|admin| admin == &admin_to_remove) {
        if relayer.admins.len() <= 1 {
            return Err(RelayerError::LastAdmin);
        }
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
    if new_amount < 10_000_000_000_000_000_000_000 { // e.g., 0.01 NEAR
        return Err(RelayerError::AmountTooLow);
    }
    relayer.sponsor_amount = new_amount;
    RelayerEvent::SponsorAmountUpdated { new_amount }.emit();
    Ok(())
}

pub fn set_max_gas_pool(relayer: &mut Relayer, new_max: u128) -> Result<(), RelayerError> {
    let caller = env::predecessor_account_id();
    if !relayer.is_admin(&caller) {
        return Err(RelayerError::Unauthorized);
    }
    if new_max < relayer.min_gas_pool {
        return Err(RelayerError::AmountTooLow);
    }
    relayer.max_gas_pool = new_max;
    RelayerEvent::MaxGasPoolUpdated { new_max }.emit();
    Ok(())
}

pub fn set_min_gas_pool(relayer: &mut Relayer, new_min: u128) -> Result<(), RelayerError> {
    let caller = env::predecessor_account_id();
    if !relayer.is_admin(&caller) {
        return Err(RelayerError::Unauthorized);
    }
    if new_min > relayer.max_gas_pool {
        return Err(RelayerError::AmountTooLow);
    }
    relayer.min_gas_pool = new_min;
    RelayerEvent::MinGasPoolUpdated { new_min }.emit();
    Ok(())
}

pub fn add_chain_mpc_mapping(relayer: &mut Relayer, chain: String, mpc_contract: AccountId) -> Result<(), RelayerError> {
    let caller = env::predecessor_account_id();
    if !relayer.is_admin(&caller) {
        return Err(RelayerError::Unauthorized);
    }
    relayer.chain_mpc_mapping.insert(chain.clone(), mpc_contract.clone());
    RelayerEvent::ChainMpcMappingAdded { chain, mpc_contract }.emit();
    Ok(())
}

pub fn remove_chain_mpc_mapping(relayer: &mut Relayer, chain: String) -> Result<(), RelayerError> {
    let caller = env::predecessor_account_id();
    if !relayer.is_admin(&caller) {
        return Err(RelayerError::Unauthorized);
    }
    relayer.chain_mpc_mapping.remove(&chain);
    RelayerEvent::ChainMpcMappingRemoved { chain }.emit();
    Ok(())
}

pub fn set_chunk_size(relayer: &mut Relayer, new_size: usize) -> Result<(), RelayerError> {
    let caller = env::predecessor_account_id();
    if !relayer.is_admin(&caller) {
        return Err(RelayerError::Unauthorized);
    }
    if new_size < 1 || new_size > 100 {
        return Err(RelayerError::AmountTooLow);
    }
    relayer.chunk_size = new_size;
    RelayerEvent::ChunkSizeUpdated { new_size }.emit();
    Ok(())
}

pub fn set_max_gas(relayer: &mut Relayer, new_max: Gas) -> Result<(), RelayerError> {
    let caller = env::predecessor_account_id();
    if !relayer.is_admin(&caller) {
        return Err(RelayerError::Unauthorized);
    }
    if new_max.as_tgas() < 50 {
        return Err(RelayerError::AmountTooLow);
    }
    relayer.max_gas = new_max;
    RelayerEvent::MaxGasUpdated { new_max: new_max.as_tgas() }.emit();
    Ok(())
}

pub fn set_mpc_sign_gas(relayer: &mut Relayer, new_gas: Gas) -> Result<(), RelayerError> {
    let caller = env::predecessor_account_id();
    if !relayer.is_admin(&caller) {
        return Err(RelayerError::Unauthorized);
    }
    if new_gas.as_tgas() < 20 {
        return Err(RelayerError::AmountTooLow);
    }
    relayer.mpc_sign_gas = new_gas;
    RelayerEvent::MpcSignGasUpdated { new_gas: new_gas.as_tgas() }.emit();
    Ok(())
}

pub fn set_callback_gas(relayer: &mut Relayer, new_gas: Gas) -> Result<(), RelayerError> {
    let caller = env::predecessor_account_id();
    if !relayer.is_admin(&caller) {
        return Err(RelayerError::Unauthorized);
    }
    if new_gas.as_tgas() < 5 {
        return Err(RelayerError::AmountTooLow);
    }
    relayer.callback_gas = new_gas;
    RelayerEvent::CallbackGasUpdated { new_gas: new_gas.as_tgas() }.emit();
    Ok(())
}

pub fn set_registrar(relayer: &mut Relayer, new_registrar: AccountId) -> Result<(), RelayerError> {
    let caller = env::predecessor_account_id();
    if !relayer.is_admin(&caller) {
        return Err(RelayerError::Unauthorized);
    }
    relayer.registrar = new_registrar.clone();
    RelayerEvent::RegistrarUpdated { new_registrar }.emit();
    Ok(())
}

pub fn pause(relayer: &mut Relayer) -> Result<(), RelayerError> {
    let caller = env::predecessor_account_id();
    if !relayer.is_admin(&caller) {
        return Err(RelayerError::Unauthorized);
    }
    if relayer.paused {
        return Ok(());
    }
    relayer.paused = true;
    RelayerEvent::ContractPaused.emit();
    Ok(())
}

pub fn unpause(relayer: &mut Relayer) -> Result<(), RelayerError> {
    let caller = env::predecessor_account_id();
    if !relayer.is_admin(&caller) {
        return Err(RelayerError::Unauthorized);
    }
    if !relayer.paused {
        return Ok(());
    }
    relayer.paused = false;
    RelayerEvent::ContractUnpaused.emit();
    Ok(())
}

pub fn migrate(relayer: &mut Relayer) -> Result<(), RelayerError> {
    let caller = env::predecessor_account_id();
    if !relayer.is_admin(&caller) {
        return Err(RelayerError::Unauthorized);
    }
    if !relayer.paused {
        return Err(RelayerError::ContractPaused);
    }

    match relayer.version.as_str() {
        "1.0" => {
            relayer.version = "1.1".to_string();
            RelayerEvent::MigrationCompleted { 
                from_version: "1.0".to_string(), 
                to_version: "1.1".to_string() 
            }.emit();
            Ok(())
        }
        "1.1" => Ok(()),
        _ => Err(RelayerError::InvalidNonce),
    }
}