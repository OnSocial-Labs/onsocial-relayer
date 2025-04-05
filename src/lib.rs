use near_sdk::{near, AccountId, Promise, PublicKey, NearToken, env};
use near_sdk::json_types::U128;
use crate::state::Relayer;
use crate::types::SignedDelegateAction;
use crate::errors::RelayerError;

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
    pub fn deposit(&mut self) {
        let deposit = env::attached_deposit().as_yoctonear();
        self.relayer.gas_pool += deposit;
        if self.relayer.gas_pool > self.relayer.max_gas_pool {
            let excess = self.relayer.gas_pool - self.relayer.max_gas_pool;
            self.relayer.gas_pool = self.relayer.max_gas_pool;
            Promise::new(self.relayer.offload_recipient.clone())
                .transfer(NearToken::from_yoctonear(excess));
        }
    }

    #[payable]
    #[handle_result]
    pub fn deposit_gas_pool(&mut self) -> Result<(), RelayerError> {
        gas_pool::deposit_gas_pool(&mut self.relayer)
    }

    #[handle_result]
    pub fn relay_meta_transaction(&mut self, #[serializer(borsh)] signed_delegate: SignedDelegateAction) -> Result<Promise, RelayerError> {
        relay::relay_meta_transaction(&mut self.relayer, signed_delegate)
    }

    #[handle_result]
    pub fn relay_meta_transactions(&mut self, #[serializer(borsh)] signed_delegates: Vec<SignedDelegateAction>) -> Result<Vec<Promise>, RelayerError> {
        relay::relay_meta_transactions(&mut self.relayer, signed_delegates)
    }

    #[handle_result]
    pub fn relay_chunked_meta_transactions(&mut self, #[serializer(borsh)] signed_delegates: Vec<SignedDelegateAction>) -> Result<Vec<Promise>, RelayerError> {
        relay::relay_chunked_meta_transactions(&mut self.relayer, signed_delegates)
    }

    #[handle_result]
    pub fn sponsor_account(&mut self, account_name: String, public_key: PublicKey) -> Result<Promise, RelayerError> {
        sponsor::sponsor_account(&mut self.relayer, account_name, public_key)
    }

    #[handle_result]
    pub fn add_auth_account(&mut self, auth_account: AccountId, auth_public_key: PublicKey) -> Result<(), RelayerError> {
        admin::add_auth_account(&mut self.relayer, auth_account, auth_public_key)
    }

    #[handle_result]
    pub fn remove_auth_account(&mut self, auth_account: AccountId) -> Result<(), RelayerError> {
        admin::remove_auth_account(&mut self.relayer, auth_account)
    }

    #[handle_result]
    pub fn set_offload_recipient(&mut self, new_recipient: AccountId) -> Result<(), RelayerError> {
        admin::set_offload_recipient(&mut self.relayer, new_recipient)
    }

    #[handle_result]
    pub fn add_admin(&mut self, new_admin: AccountId) -> Result<(), RelayerError> {
        admin::add_admin(&mut self.relayer, new_admin)
    }

    #[handle_result]
    pub fn remove_admin(&mut self, admin_to_remove: AccountId) -> Result<(), RelayerError> {
        admin::remove_admin(&mut self.relayer, admin_to_remove)
    }

    #[handle_result]
    pub fn set_sponsor_amount(&mut self, new_amount: U128) -> Result<(), RelayerError> {
        admin::set_sponsor_amount(&mut self.relayer, new_amount.0)
    }

    #[handle_result]
    pub fn set_max_gas_pool(&mut self, new_max: U128) -> Result<(), RelayerError> {
        admin::set_max_gas_pool(&mut self.relayer, new_max.0)
    }

    #[handle_result]
    pub fn set_min_gas_pool(&mut self, new_min: U128) -> Result<(), RelayerError> {
        admin::set_min_gas_pool(&mut self.relayer, new_min.0)
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

    // Callback to handle gas refunds
    #[private]
    pub fn refund_gas_callback(&mut self, initial_cost: u128) {
        let used_gas = env::used_gas().as_tgas() as u128; // Gas actually used in TGas
        let gas_price = 100_000_000_000; // Min gas price: 0.0001 Ⓝ/TGas in yoctoNEAR
        let actual_cost = used_gas * gas_price; // Approximate cost in yoctoNEAR
        let refund = initial_cost.saturating_sub(actual_cost);
        self.relayer.gas_pool += refund;

        if self.relayer.gas_pool > self.relayer.max_gas_pool {
            let excess = self.relayer.gas_pool - self.relayer.max_gas_pool;
            self.relayer.gas_pool = self.relayer.max_gas_pool;
            Promise::new(self.relayer.offload_recipient.clone())
                .transfer(NearToken::from_yoctonear(excess));
        }
    }
}

#[cfg(test)]
mod tests;