use near_sdk::{env, AccountId, Promise, PublicKey};
use near_sdk::json_types::U128;
use serde_json;
use core::num::NonZeroU128;
use crate::state::Relayer;
use crate::errors::RelayerError;
use crate::events::RelayerEvent;

impl Relayer {
    pub fn sponsor_account(
        &mut self,
        account_name: String,
        public_key: PublicKey,
        add_function_call_key: bool,
        is_implicit: bool,
    ) -> Result<Promise, RelayerError> {
        let new_account_id: AccountId = if is_implicit {
            let account_str = account_name.to_lowercase();
            if account_str.len() != 64 || !account_str.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(RelayerError::InvalidAccountId);
            }
            account_str.parse().map_err(|_| RelayerError::InvalidAccountId)?
        } else {
            let is_testnet = env::current_account_id().as_str().contains(".testnet");
            format!("{}.{}", account_name, if is_testnet { "testnet" } else { "near" })
                .parse()
                .map_err(|_| RelayerError::InvalidAccountId)?
        };

        if self.processed_nonces.contains_key(&new_account_id) {
            return Err(RelayerError::AccountExists);
        }
        if env::account_balance() < self.min_gas_pool.checked_add(self.sponsor_amount).unwrap() {
            return Err(RelayerError::InsufficientBalance);
        }

        let mut promise = if is_implicit {
            Promise::new(new_account_id.clone()).transfer(self.sponsor_amount)
        } else {
            let root_account = if env::current_account_id().as_str().contains(".testnet") { "testnet" } else { "near" }
                .parse()
                .unwrap();
            Promise::new(root_account).function_call(
                "create_account".to_string(),
                serde_json::to_vec(&serde_json::json!({
                    "new_account_id": new_account_id.as_str(),
                    "new_public_key": public_key
                })).unwrap(),
                self.sponsor_amount,
                self.default_gas,
            )
        };

        if add_function_call_key {
            let mut fc_key_bytes = vec![0];
            fc_key_bytes.extend_from_slice(&[2; 32]);
            let fc_key = PublicKey::try_from(fc_key_bytes).unwrap();
            promise = promise.then(Promise::new(new_account_id.clone()).add_access_key_allowance(
                fc_key.clone(),
                near_sdk::Allowance::Limited(NonZeroU128::new(100_000_000_000_000_000_000_000).unwrap()),
                env::current_account_id(),
                "relay_meta_transaction".to_string(),
            ));
            RelayerEvent::FunctionCallKeyAdded {
                account_id: new_account_id.clone(),
                public_key: fc_key,
                receiver_id: env::current_account_id(),
            }.emit();
        }

        RelayerEvent::AccountSponsored { account_id: new_account_id, public_key, is_implicit }.emit();
        Ok(promise)
    }

    pub fn get_sponsor_amount(&self) -> U128 {
        U128(self.sponsor_amount.as_yoctonear())
    }
}