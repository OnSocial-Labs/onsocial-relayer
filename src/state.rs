use near_sdk::{AccountId, Gas, NearToken};
use near_sdk::store::{LookupMap, Vector};
use near_sdk::json_types::U128;
use near_sdk::borsh::{BorshSerialize, BorshDeserialize};
use crate::types::{SignedDelegateAction, SerializablePromiseResult};

pub const FAILED_TX_QUEUE_CAP: u32 = 100;

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Relayer {
    pub gas_pool: NearToken,
    pub min_gas_pool: NearToken,
    pub sponsor_amount: NearToken,
    pub payment_ft_contract: Option<AccountId>,
    pub min_ft_payment: NearToken,
    pub whitelisted_contracts: Vector<AccountId>,
    pub processed_nonces: LookupMap<AccountId, u64>,
    pub failed_transactions: Vector<(SignedDelegateAction, Gas)>,
    pub default_gas: Gas,
    pub gas_buffer: Gas,
    pub admins: Vector<AccountId>,
    #[cfg(test)]
    pub simulate_signature_failure: bool,
    #[cfg(test)]
    pub simulate_promise_result: Option<SerializablePromiseResult>,
}

impl Relayer {
    pub fn new(payment_ft_contract: Option<AccountId>, min_ft_payment: U128, whitelisted_contracts: Vec<AccountId>) -> Self {
        let mut whitelisted = Vector::new(b"w".to_vec());
        for contract in whitelisted_contracts {
            whitelisted.push(contract);
        }
        whitelisted.push("social.near".parse().unwrap());
        whitelisted.push("social.tkn.near".parse().unwrap());
        whitelisted.push("3e2210e1184b45b64c8a434c0a7e7b23cc04ea7eb7a6c3c32520d03d4afcb8af".parse().unwrap());
        whitelisted.push("17208628f84f5d6ad33f0da3bbbeb27ffcb398eac501a31bd6ad2011e36133a1".parse().unwrap());

        let mut admins = Vector::new(b"a".to_vec());
        admins.push("onsocial.sputnik-dao.near".parse().unwrap());
        admins.push("onsocial.testnet".parse().unwrap());
        admins.push("onsocial.near".parse().unwrap());

        Self {
            gas_pool: NearToken::from_yoctonear(0),
            min_gas_pool: NearToken::from_yoctonear(1_000_000_000_000_000_000_000_000),
            sponsor_amount: NearToken::from_yoctonear(100_000_000_000_000_000_000_000),
            payment_ft_contract,
            min_ft_payment: NearToken::from_yoctonear(min_ft_payment.0),
            whitelisted_contracts: whitelisted,
            processed_nonces: LookupMap::new(b"n".to_vec()),
            failed_transactions: Vector::new(b"f".to_vec()),
            default_gas: Gas::from_tgas(150),
            gas_buffer: Gas::from_tgas(50),
            admins,
            #[cfg(test)]
            simulate_signature_failure: false,
            #[cfg(test)]
            simulate_promise_result: None,
        }
    }

    pub fn clean_failed_transactions(&mut self) {
        let current_height = near_sdk::env::block_height();
        let mut new_queue = Vector::new(b"f".to_vec());

        for (signed_delegate, gas) in self.failed_transactions.iter() {
            if signed_delegate.delegate_action.max_block_height >= current_height {
                if new_queue.len() < FAILED_TX_QUEUE_CAP {
                    new_queue.push((signed_delegate.clone(), *gas));
                } else {
                    near_sdk::env::log_str(&format!(
                        "Dropped transaction with nonce {} due to queue cap",
                        signed_delegate.delegate_action.nonce
                    ));
                }
            }
        }

        self.failed_transactions = new_queue;
    }
}