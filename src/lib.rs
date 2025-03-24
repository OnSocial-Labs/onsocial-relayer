use near_sdk::{near, PanicOnDefault};

pub mod types;
pub mod errors;
pub mod events;
pub mod state;
pub mod gas_pool;
pub mod sponsor;
pub mod meta_tx;
pub mod admin;
pub mod tests;

pub use crate::state::Relayer;

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Contract {
    relayer: Relayer,
}

#[near]
impl Contract {
    #[init]
    pub fn new(payment_ft_contract: Option<near_sdk::AccountId>, min_ft_payment: near_sdk::json_types::U128, whitelisted_contracts: Vec<near_sdk::AccountId>) -> Self {
        Self {
            relayer: Relayer::new(payment_ft_contract, min_ft_payment, whitelisted_contracts),
        }
    }

    #[payable]
    pub fn deposit_gas_pool(&mut self) {
        self.relayer.deposit_gas_pool();
    }

    pub fn get_gas_pool(&self) -> near_sdk::json_types::U128 {
        self.relayer.get_gas_pool()
    }
}