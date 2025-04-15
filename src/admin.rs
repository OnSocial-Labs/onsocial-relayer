use near_sdk::{env, AccountId, PublicKey, Gas};
use crate::{ext_auth, state::Relayer};
use crate::errors::RelayerError;
use crate::events::RelayerEvent;

pub fn register_existing_account(
    relayer: &mut Relayer,
    account_id: AccountId,
    public_key: PublicKey,
    expiration_days: Option<u32>,
    is_multi_sig: bool,
    multi_sig_threshold: Option<u32>,
) -> Result<(), RelayerError> {
    if relayer.paused {
        return Err(RelayerError::ContractPaused);
    }

    let caller = env::predecessor_account_id();
    if caller != account_id {
        return Err(RelayerError::Unauthorized);
    }

    if public_key.as_bytes().len() != 33 || public_key.as_bytes()[0] != 0 {
        return Err(RelayerError::InvalidSignature);
    }

    ext_auth::ext(relayer.auth_contract.clone())
        .with_static_gas(Gas::from_tgas(relayer.cross_contract_gas))
        .register_key(
            account_id.clone(),
            public_key.clone(),
            expiration_days,
            is_multi_sig,
            multi_sig_threshold,
        );

    let key_hash = hex::encode(env::sha256(&public_key.as_bytes()));
    RelayerEvent::AuthAdded { auth_account: account_id, key_hash }.emit();
    Ok(())
}

pub fn remove_key(relayer: &mut Relayer, account_id: AccountId, public_key: PublicKey) -> Result<(), RelayerError> {
    if relayer.paused {
        return Err(RelayerError::ContractPaused);
    }

    let caller = env::predecessor_account_id();
    if caller != account_id {
        return Err(RelayerError::Unauthorized);
    }

    ext_auth::ext(relayer.auth_contract.clone())
        .with_static_gas(Gas::from_tgas(relayer.cross_contract_gas))
        .remove_key(account_id.clone(), public_key.clone());

    let key_hash = hex::encode(env::sha256(&public_key.as_bytes()));
    RelayerEvent::AuthRemoved { auth_account: account_id, key_hash }.emit();
    Ok(())
}

pub fn set_cross_contract_gas(relayer: &mut Relayer, new_gas: u64) -> Result<(), RelayerError> {
    let caller = env::predecessor_account_id();
    if !relayer.is_admin(&caller) {
        return Err(RelayerError::Unauthorized);
    }
    if new_gas < 10_000_000_000_000 || new_gas > 300_000_000_000_000 {
        return Err(RelayerError::AmountTooLow);
    }
    relayer.cross_contract_gas = new_gas;
    RelayerEvent::CrossContractGasUpdated { new_gas }.emit();
    Ok(())
}

pub fn set_omni_locker_contract(relayer: &mut Relayer, new_locker_contract: AccountId) -> Result<(), RelayerError> {
    let caller = env::predecessor_account_id();
    if !relayer.is_admin(&caller) {
        return Err(RelayerError::Unauthorized);
    }
    relayer.omni_locker_contract.set(Some(new_locker_contract.clone()));
    RelayerEvent::OmniLockerContractUpdated { new_locker_contract }.emit();
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
    if !relayer.admins.iter().any(|admin| admin == &new_admin) {
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
        relayer.admins.swap_remove(index.try_into().unwrap());
        RelayerEvent::AdminRemoved { admin_account: admin_to_remove }.emit();
    }
    Ok(())
}

pub fn set_sponsor_amount(relayer: &mut Relayer, new_amount: u128) -> Result<(), RelayerError> {
    let caller = env::predecessor_account_id();
    if !relayer.is_admin(&caller) {
        return Err(RelayerError::Unauthorized);
    }
    if new_amount < 10_000_000_000_000_000_000_000 {
        return Err(RelayerError::AmountTooLow);
    }
    relayer.sponsor_amount = new_amount;
    RelayerEvent::SponsorAmountUpdated { new_amount }.emit();
    Ok(())
}

pub fn set_sponsor_gas(relayer: &mut Relayer, new_gas: u64) -> Result<(), RelayerError> {
    let caller = env::predecessor_account_id();
    if !relayer.is_admin(&caller) {
        return Err(RelayerError::Unauthorized);
    }
    if new_gas < 50_000_000_000_000 || new_gas > 300_000_000_000_000 {
        return Err(RelayerError::AmountTooLow);
    }
    relayer.sponsor_gas = new_gas;
    RelayerEvent::SponsorGasUpdated { new_gas }.emit();
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

pub fn set_auth_contract(relayer: &mut Relayer, new_auth_contract: AccountId) -> Result<(), RelayerError> {
    let caller = env::predecessor_account_id();
    if !relayer.is_admin(&caller) {
        return Err(RelayerError::Unauthorized);
    }
    relayer.auth_contract = new_auth_contract.clone();
    RelayerEvent::AuthContractUpdated { new_auth_contract }.emit();
    Ok(())
}

pub fn set_ft_wrapper_contract(relayer: &mut Relayer, new_ft_wrapper_contract: AccountId) -> Result<(), RelayerError> {
    let caller = env::predecessor_account_id();
    if !relayer.is_admin(&caller) {
        return Err(RelayerError::Unauthorized);
    }
    relayer.ft_wrapper_contract = new_ft_wrapper_contract.clone();
    RelayerEvent::FtWrapperContractUpdated { new_ft_wrapper_contract }.emit();
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

pub fn migrate(relayer: &mut Relayer, target_version: u64, require_pause: bool) -> Result<(), RelayerError> {
    let caller = env::predecessor_account_id();
    if !relayer.is_admin(&caller) {
        return Err(RelayerError::Unauthorized);
    }
    if require_pause && !relayer.paused {
        return Err(RelayerError::ContractPaused);
    }
    if target_version <= relayer.migration_version {
        return Err(RelayerError::InvalidNonce);
    }

    let current = relayer.migration_version;
    for version in (current + 1)..=target_version {
        match version {
            1 => {
                relayer.version = "1.1".to_string();
            }
            _ => return Err(RelayerError::InvalidNonce),
        }
        relayer.migration_version = version;
        RelayerEvent::MigrationCompleted { 
            from_version: version - 1, 
            to_version: version 
        }.emit();
    }
    Ok(())
}

pub fn set_min_balance(relayer: &mut Relayer, new_min: u128) -> Result<(), RelayerError> {
    let caller = env::predecessor_account_id();
    if !relayer.is_admin(&caller) {
        return Err(RelayerError::Unauthorized);
    }
    if new_min > relayer.max_balance {
        return Err(RelayerError::AmountTooLow);
    }
    relayer.min_balance = new_min;
    RelayerEvent::MinBalanceUpdated { new_min }.emit();
    Ok(())
}

pub fn set_max_balance(relayer: &mut Relayer, new_max: u128) -> Result<(), RelayerError> {
    let caller = env::predecessor_account_id();
    if !relayer.is_admin(&caller) {
        return Err(RelayerError::Unauthorized);
    }
    if new_max < relayer.min_balance {
        return Err(RelayerError::AmountTooLow);
    }
    relayer.max_balance = new_max;
    RelayerEvent::MaxBalanceUpdated { new_max }.emit();
    Ok(())
}

pub fn set_base_fee(relayer: &mut Relayer, new_fee: u128) -> Result<(), RelayerError> {
    let caller = env::predecessor_account_id();
    if !relayer.is_admin(&caller) {
        return Err(RelayerError::Unauthorized);
    }
    if new_fee < 100_000_000_000_000_000_000 {
        return Err(RelayerError::AmountTooLow);
    }
    relayer.base_fee = new_fee;
    RelayerEvent::BaseFeeUpdated { new_fee }.emit();
    Ok(())
}
