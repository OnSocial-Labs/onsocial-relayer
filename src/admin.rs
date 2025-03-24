use near_sdk::{env, AccountId, Gas, NearToken, PublicKey};
use near_sdk::json_types::U128;
use near_sdk::store::Vector;
use crate::state::Relayer;
use crate::errors::RelayerError;
use crate::events::RelayerEvent;
use crate::types::{Action, DelegateAction, SignedDelegateAction};

impl Relayer {
    pub fn update_whitelist(&mut self, contracts: Vec<AccountId>) -> Result<(), RelayerError> {
        let caller = env::predecessor_account_id();
        if !self.admins.iter().any(|admin| admin == &caller) {
            return Err(RelayerError::Unauthorized);
        }
        self.whitelisted_contracts.clear();
        for contract in contracts {
            self.whitelisted_contracts.push(contract);
        }
        env::log_str(&format!("Whitelist updated with {} contracts by {}", self.whitelisted_contracts.len(), caller.as_str()));
        Ok(())
    }

    pub fn set_sponsor_amount(&mut self, amount: U128) -> Result<(), RelayerError> {
        let caller = env::predecessor_account_id();
        if !self.admins.iter().any(|admin| admin == &caller) {
            return Err(RelayerError::Unauthorized);
        }
        if amount.0 < 50_000_000_000_000_000_000_000 {
            return Err(RelayerError::InvalidSponsorAmount);
        }
        self.sponsor_amount = NearToken::from_yoctonear(amount.0);
        env::log_str(&format!("Sponsor amount updated to {} yoctoNEAR by {}", amount.0, caller.as_str()));
        Ok(())
    }

    pub fn set_admins(&mut self, new_admins: Vec<AccountId>) -> Result<(), RelayerError> {
        let caller = env::predecessor_account_id();
        if !self.admins.iter().any(|admin| admin == &caller) {
            return Err(RelayerError::Unauthorized);
        }
        if new_admins.is_empty() {
            return Err(RelayerError::InvalidAccountId);
        }
        self.admins.clear();
        for admin in new_admins {
            self.admins.push(admin);
        }
        env::log_str(&format!("Admins updated to {} accounts by {}", self.admins.len(), caller.as_str()));
        Ok(())
    }

    pub fn add_function_call_key(
        &mut self,
        account_id: AccountId,
        public_key: PublicKey,
        receiver_id: AccountId,
        method_names: Vec<String>,
    ) -> Result<(), RelayerError> {
        let caller = env::predecessor_account_id();
        if !self.admins.iter().any(|admin| admin == &caller) {
            return Err(RelayerError::Unauthorized);
        }
        let action = Action::AddKey {
            public_key,
            allowance: Some(NearToken::from_yoctonear(100_000_000_000_000_000_000_000)),
            receiver_id: receiver_id.clone(),
            method_names,
        };
        let delegate = DelegateAction {
            sender_id: account_id.clone(),
            receiver_id,
            actions: vec![action],
            nonce: *self.processed_nonces.get(&account_id).unwrap_or(&0) + 1,
            max_block_height: env::block_height() + 100,
        };
        let mut pk_bytes = vec![0];
        pk_bytes.extend_from_slice(&[0; 32]);
        let dummy_pk = PublicKey::try_from(pk_bytes).unwrap();
        let signed_delegate = SignedDelegateAction {
            delegate_action: delegate,
            signature: vec![0; 64],
            public_key: dummy_pk,
        };
        self.relay_meta_transaction(signed_delegate)
    }

    pub fn set_gas_config(&mut self, default_gas_tgas: u64, gas_buffer_tgas: u64) -> Result<(), RelayerError> {
        let caller = env::predecessor_account_id();
        if !self.admins.iter().any(|admin| admin == &caller) {
            return Err(RelayerError::Unauthorized);
        }
        if default_gas_tgas < 50 || gas_buffer_tgas < 10 {
            return Err(RelayerError::InvalidGasConfig);
        }
        self.default_gas = Gas::from_tgas(default_gas_tgas);
        self.gas_buffer = Gas::from_tgas(gas_buffer_tgas);
        env::log_str(&format!("Gas config updated: default={} TGas, buffer={} TGas by {}", default_gas_tgas, gas_buffer_tgas, caller.as_str()));
        Ok(())
    }

    pub fn retry_or_clear_failed_transactions(&mut self, retry: bool) -> Result<(), RelayerError> {
        let caller = env::predecessor_account_id();
        if !self.admins.iter().any(|admin| admin == &caller) {
            return Err(RelayerError::Unauthorized);
        }
        if self.failed_transactions.is_empty() {
            return Err(RelayerError::NoFailedTransactions);
        }

        self.clean_failed_transactions();
        let count = self.failed_transactions.len();

        if retry {
            let mut failed = Vector::new(b"f".to_vec());
            std::mem::swap(&mut self.failed_transactions, &mut failed);
            let mut retry_count = 0;
            for (signed_delegate, gas) in failed.iter() {
                match self.relay_meta_transaction(signed_delegate.clone()) {
                    Ok(()) => {
                        retry_count += 1;
                        env::log_str(&format!("Manual retry of transaction with nonce {} succeeded", signed_delegate.delegate_action.nonce));
                    }
                    Err(_e) => {
                        if self.failed_transactions.len() < crate::state::FAILED_TX_QUEUE_CAP {
                            self.failed_transactions.push((signed_delegate.clone(), gas.saturating_add(self.gas_buffer)));
                            env::log_str(&format!("Manual retry failed: {:?}", _e));
                        }
                    }
                }
            }
            if retry_count > 0 {
                RelayerEvent::FailedTransactionsRetried { count: retry_count }.emit();
            }
        } else {
            self.failed_transactions.clear();
            RelayerEvent::FailedTransactionsCleared { count }.emit();
        }
        Ok(())
    }

    pub fn get_admins(&self) -> Vec<AccountId> {
        self.admins.iter().cloned().collect()
    }

    pub fn get_default_gas(&self) -> Gas {
        self.default_gas
    }

    pub fn get_gas_buffer(&self) -> Gas {
        self.gas_buffer
    }

    pub fn get_failed_transactions_count(&self) -> u32 {
        self.failed_transactions.len()
    }

    #[cfg(test)]
    pub fn set_simulate_signature_failure(&mut self, fail: bool) {
        self.simulate_signature_failure = fail;
    }

    #[cfg(test)]
    pub fn set_simulate_promise_result(&mut self, result: Option<crate::types::SerializablePromiseResult>) {
        self.simulate_promise_result = result;
    }
}