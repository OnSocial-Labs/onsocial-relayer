use near_sdk::{AccountId};
use near_sdk::borsh::{self, BorshSchema, BorshSerialize, BorshDeserialize};
use std::collections::HashMap;
use crate::types::{SignedDelegateAction, WrappedAccountId};

pub const FAILED_TX_QUEUE_CAP: u32 = 100;

#[derive(BorshSerialize, BorshDeserialize, BorshSchema, Clone, PartialEq, Eq, Hash, Ord, PartialOrd, Debug)]
pub struct AccountIdWrapper(pub WrappedAccountId);

impl From<WrappedAccountId> for AccountIdWrapper {
    fn from(account_id: WrappedAccountId) -> Self {
        Self(account_id)
    }
}

impl From<AccountIdWrapper> for WrappedAccountId {
    fn from(wrapper: AccountIdWrapper) -> Self {
        wrapper.0
    }
}

#[derive(BorshSerialize, BorshDeserialize, BorshSchema)]
pub struct Relayer {
    pub gas_pool: u128,
    pub min_gas_pool: u128,
    pub sponsor_amount: u128,
    pub payment_ft_contract: Option<AccountIdWrapper>,
    pub min_ft_payment: u128,
    pub whitelisted_contracts: Vec<AccountIdWrapper>,
    pub processed_nonces: HashMap<AccountIdWrapper, u64>,
    pub failed_transactions: Vec<(SignedDelegateAction, u64)>,
    pub default_gas: u64,
    pub gas_buffer: u64,
    pub admins: Vec<AccountIdWrapper>,
    #[cfg(test)]
    pub simulate_signature_failure: bool,
    #[cfg(test)]
    pub simulate_promise_result: Option<crate::types::SerializablePromiseResult>,
}

impl Relayer {
    pub fn new(payment_ft_contract: Option<AccountId>, min_ft_payment: near_sdk::json_types::U128, whitelisted_contracts: Vec<AccountId>) -> Self {
        let mut whitelisted = Vec::new();
        for contract in whitelisted_contracts {
            whitelisted.push(AccountIdWrapper::from(WrappedAccountId(contract)));
        }
        whitelisted.push(AccountIdWrapper::from(WrappedAccountId("social.near".parse::<AccountId>().unwrap())));
        whitelisted.push(AccountIdWrapper::from(WrappedAccountId("social.tkn.near".parse::<AccountId>().unwrap())));
        whitelisted.push(AccountIdWrapper::from(WrappedAccountId("3e2210e1184b45b64c8a434c0a7e7b23cc04ea7eb7a6c3c32520d03d4afcb8af".parse::<AccountId>().unwrap())));
        whitelisted.push(AccountIdWrapper::from(WrappedAccountId("17208628f84f5d6ad33f0da3bbbeb27ffcb398eac501a31bd6ad2011e36133a1".parse::<AccountId>().unwrap())));

        let mut admins = Vec::new();
        admins.push(AccountIdWrapper::from(WrappedAccountId("onsocial.sputnik-dao.near".parse::<AccountId>().unwrap())));
        admins.push(AccountIdWrapper::from(WrappedAccountId("onsocial.testnet".parse::<AccountId>().unwrap())));
        admins.push(AccountIdWrapper::from(WrappedAccountId("onsocial.near".parse::<AccountId>().unwrap())));

        Self {
            gas_pool: 0,
            min_gas_pool: 1_000_000_000_000_000_000_000_000,
            sponsor_amount: 100_000_000_000_000_000_000_000,
            payment_ft_contract: payment_ft_contract.map(|id| AccountIdWrapper::from(WrappedAccountId(id))),
            min_ft_payment: min_ft_payment.0,
            whitelisted_contracts: whitelisted,
            processed_nonces: HashMap::new(),
            failed_transactions: Vec::new(),
            default_gas: 150_000_000_000_000, // 150 TGas
            gas_buffer: 50_000_000_000_000,   // 50 TGas
            admins,
            #[cfg(test)]
            simulate_signature_failure: false,
            #[cfg(test)]
            simulate_promise_result: None,
        }
    }

    pub fn clean_failed_transactions(&mut self) {
        let current_height = near_sdk::env::block_height();
        self.failed_transactions.retain(|(signed_delegate, _)| {
            signed_delegate.delegate_action.max_block_height >= current_height
        });
        while self.failed_transactions.len() > FAILED_TX_QUEUE_CAP as usize {
            let removed = self.failed_transactions.remove(0);
            near_sdk::env::log_str(&format!(
                "Dropped transaction with nonce {} due to queue cap",
                removed.0.delegate_action.nonce
            ));
        }
    }
}