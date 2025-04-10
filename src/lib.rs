use near_sdk::{near, AccountId, Promise, PublicKey, NearToken, env, ext_contract};
use near_sdk::json_types::U128;
use near_sdk::{borsh, PanicOnDefault};
use crate::state::{Relayer, RelayerV1};
use crate::types::SignedDelegateAction;
use crate::errors::RelayerError;
use crate::events::RelayerEvent;

mod types;
mod errors;
mod events;
mod state;
mod admin;
mod relay;
mod sponsor;
mod gas_pool;

#[ext_contract(ext_self)]
pub trait SelfCallback {
    fn refund_gas_callback(&mut self, initial_cost: u128);
    fn handle_mpc_signature(&mut self, chain: String, request_id: u64, result: Vec<u8>);
    fn handle_bridge_result(&mut self, sender_id: AccountId, action_type: String, result: Vec<u8>);
}

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct OnSocialRelayer {
    relayer: Relayer,
}

#[near]
impl OnSocialRelayer {
    #[init]
    pub fn new(admins: Vec<AccountId>, initial_auth_account: AccountId, initial_auth_key: String, offload_recipient: AccountId) -> Self {
        let initial_auth_key: PublicKey = initial_auth_key.parse().expect("Invalid public key");
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
    pub fn sponsor_account(&mut self, #[serializer(borsh)] args: Vec<u8>) -> Result<Promise, RelayerError> {
        env::log_str(&format!("Raw args: {:?}", args));
        let (new_account_id, public_key): (AccountId, PublicKey) = borsh::from_slice(&args)
            .map_err(|e| {
                env::log_str(&format!("Deserialization failed: {:?}", e));
                RelayerError::InvalidNonce
            })?;
        env::log_str(&format!("Deserialized: {} {:?}", new_account_id, public_key));
        sponsor::sponsor_account(&mut self.relayer, new_account_id, public_key)
    }

    #[handle_result]
    pub fn sponsor_account_signed(&mut self, #[serializer(borsh)] signed_delegate: SignedDelegateAction) -> Result<Promise, RelayerError> {
        sponsor::sponsor_account_signed(&mut self.relayer, signed_delegate)
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

    #[handle_result]
    pub fn add_chain_mpc_mapping(&mut self, chain: String, mpc_contract: AccountId) -> Result<(), RelayerError> {
        admin::add_chain_mpc_mapping(&mut self.relayer, chain, mpc_contract)
    }

    #[handle_result]
    pub fn remove_chain_mpc_mapping(&mut self, chain: String) -> Result<(), RelayerError> {
        admin::remove_chain_mpc_mapping(&mut self.relayer, chain)
    }

    #[handle_result]
    pub fn set_chunk_size(&mut self, new_size: usize) -> Result<(), RelayerError> {
        admin::set_chunk_size(&mut self.relayer, new_size)
    }

    #[handle_result]
    pub fn set_max_gas(&mut self, new_max: U128) -> Result<(), RelayerError> {
        admin::set_max_gas(&mut self.relayer, new_max.0)
    }

    #[handle_result]
    pub fn set_mpc_sign_gas(&mut self, new_gas: U128) -> Result<(), RelayerError> {
        admin::set_mpc_sign_gas(&mut self.relayer, new_gas.0)
    }

    #[handle_result]
    pub fn set_callback_gas(&mut self, new_gas: U128) -> Result<(), RelayerError> {
        admin::set_callback_gas(&mut self.relayer, new_gas.0)
    }

    #[handle_result]
    pub fn set_registrar(&mut self, new_registrar: AccountId) -> Result<(), RelayerError> {
        admin::set_registrar(&mut self.relayer, new_registrar)
    }

    #[handle_result]
    pub fn pause(&mut self) -> Result<(), RelayerError> {
        admin::pause(&mut self.relayer)
    }

    #[handle_result]
    pub fn unpause(&mut self) -> Result<(), RelayerError> {
        admin::unpause(&mut self.relayer)
    }

    #[handle_result]
    pub fn migrate(&mut self) -> Result<(), RelayerError> {
        admin::migrate(&mut self.relayer)
    }

    #[handle_result]
    pub fn set_gas_price(&mut self, new_gas_price: U128) -> Result<(), RelayerError> {
        admin::set_gas_price(&mut self.relayer, new_gas_price.0)
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

    pub fn get_chunk_size(&self) -> usize {
        self.relayer.chunk_size
    }

    pub fn get_max_gas(&self) -> U128 {
        U128(self.relayer.max_gas.as_gas() as u128)
    }

    pub fn get_mpc_sign_gas(&self) -> U128 {
        U128(self.relayer.mpc_sign_gas.as_gas() as u128)
    }

    pub fn get_callback_gas(&self) -> U128 {
        U128(self.relayer.callback_gas.as_gas() as u128)
    }

    pub fn get_registrar(&self) -> AccountId {
        self.relayer.registrar.clone()
    }

    pub fn is_paused(&self) -> bool {
        self.relayer.paused
    }

    pub fn get_version(&self) -> String {
        self.relayer.version.clone()
    }

    pub fn is_authorized(&self, account_id: AccountId) -> bool {
        self.relayer.auth_accounts.contains_key(&account_id)
    }

    pub fn get_gas_price(&self) -> U128 {
        U128(self.relayer.gas_price)
    }
}

#[near]
impl OnSocialRelayer {
    #[private]
    pub fn refund_gas_callback(&mut self, initial_cost: u128) {
        let used_gas = env::used_gas().as_tgas() as u128;
        let gas_price = self.relayer.gas_price;
        let actual_cost = used_gas * gas_price;
        let refund = initial_cost.saturating_sub(actual_cost);
        self.relayer.gas_pool += refund;

        if self.relayer.gas_pool > self.relayer.max_gas_pool {
            let excess = self.relayer.gas_pool - self.relayer.max_gas_pool;
            self.relayer.gas_pool = self.relayer.max_gas_pool;
            Promise::new(self.relayer.offload_recipient.clone())
                .transfer(NearToken::from_yoctonear(excess));
        }
    }

    #[private]
    pub fn handle_mpc_signature(&mut self, chain: String, request_id: u64, result: Vec<u8>) {
        RelayerEvent::CrossChainSignatureResult { chain, request_id, result }.emit();
    }

    #[private]
    pub fn handle_bridge_result(&mut self, sender_id: AccountId, action_type: String, result: Vec<u8>) {
        RelayerEvent::BridgeResult { sender_id, action_type, result }.emit();
    }

    #[init]
    #[private]
    pub fn migrate_state() -> Self {
        let old_state: RelayerV1 = env::state_read().expect("Failed to read old state");
        Self {
            relayer: Relayer::from(old_state),
        }
    }
}

#[cfg(test)]
mod tests;