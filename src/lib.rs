use near_sdk::{near, AccountId, Promise, PublicKey};
use near_sdk::json_types::U128;
use crate::state::Relayer;
use crate::types::SignedDelegateAction;

mod types;
mod errors;
mod events;
mod state;
mod admin;
mod relay;
mod sponsor;
mod gas_pool;

#[near(contract_state)]
pub struct OnSocialRelayer {
    relayer: Relayer,
}

impl Default for OnSocialRelayer {
    fn default() -> Self {
        panic!("Use `new` to initialize");
    }
}

#[near]
impl OnSocialRelayer {
    #[init]
    pub fn new(admins: Vec<AccountId>, initial_auth_account: AccountId, initial_auth_key: PublicKey, offload_recipient: AccountId) -> Self {
        Self {
            relayer: Relayer::new(admins, initial_auth_account, initial_auth_key, offload_recipient),
        }
    }

    #[payable]
    #[handle_result]
    pub fn deposit_gas_pool(&mut self) -> Result<(), errors::RelayerError> {
        gas_pool::deposit_gas_pool(&mut self.relayer)
    }

    #[handle_result]
    pub fn relay_meta_transaction(&mut self, #[serializer(borsh)] signed_delegate: SignedDelegateAction) -> Result<Promise, errors::RelayerError> {
        relay::relay_meta_transaction(&mut self.relayer, signed_delegate)
    }

    #[handle_result]
    pub fn relay_meta_transactions(&mut self, #[serializer(borsh)] signed_delegates: Vec<SignedDelegateAction>) -> Result<Vec<Promise>, errors::RelayerError> {
        relay::relay_meta_transactions(&mut self.relayer, signed_delegates)
    }

    #[handle_result]
    pub fn sponsor_account(&mut self, account_name: String, public_key: PublicKey) -> Result<Promise, errors::RelayerError> {
        sponsor::sponsor_account(&mut self.relayer, account_name, public_key)
    }

    #[handle_result]
    pub fn add_auth_account(&mut self, auth_account: AccountId, auth_public_key: PublicKey) -> Result<(), errors::RelayerError> {
        admin::add_auth_account(&mut self.relayer, auth_account, auth_public_key)
    }

    #[handle_result]
    pub fn remove_auth_account(&mut self, auth_account: AccountId) -> Result<(), errors::RelayerError> {
        admin::remove_auth_account(&mut self.relayer, auth_account)
    }

    #[handle_result]
    pub fn set_offload_recipient(&mut self, new_recipient: AccountId) -> Result<(), errors::RelayerError> {
        admin::set_offload_recipient(&mut self.relayer, new_recipient)
    }

    #[handle_result]
    pub fn add_admin(&mut self, new_admin: AccountId) -> Result<(), errors::RelayerError> {
        admin::add_admin(&mut self.relayer, new_admin)
    }

    #[handle_result]
    pub fn remove_admin(&mut self, admin_to_remove: AccountId) -> Result<(), errors::RelayerError> {
        admin::remove_admin(&mut self.relayer, admin_to_remove)
    }

    #[handle_result]
    pub fn set_sponsor_amount(&mut self, new_amount: U128) -> Result<(), errors::RelayerError> {
        admin::set_sponsor_amount(&mut self.relayer, new_amount.0)
    }

    pub fn get_gas_pool(&self) -> U128 {
        U128(self.relayer.gas_pool)
    }

    pub fn get_min_gas_pool(&self) -> U128 {
        U128(self.relayer.min_gas_pool)
    }

    pub fn get_sponsor_amount(&self) -> U128 {
        U128(self.relayer.sponsor_amount)
    }
}

#[cfg(test)]
mod tests;