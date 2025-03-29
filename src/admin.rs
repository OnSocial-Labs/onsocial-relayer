use near_sdk::{env, AccountId, Gas, NearToken, PublicKey, Promise, require};
use near_sdk::json_types::U128;
use crate::state::{Relayer, AccountIdWrapper};
use crate::errors::RelayerError;
use crate::events::RelayerEvent;
use crate::types::{Action, DelegateAction, SignedDelegateAction, WrappedAccountId, WrappedPublicKey, WrappedNearToken};

impl Relayer {
    pub fn update_whitelist(&mut self, contracts: Vec<AccountId>) -> Result<(), RelayerError> {
        let caller = env::predecessor_account_id();
        if !self.admins.iter().any(|admin| admin.0.0 == caller) {
            return Err(RelayerError::Unauthorized);
        }
        self.whitelisted_contracts.clear();
        for contract in contracts {
            self.whitelisted_contracts.push(AccountIdWrapper::from(WrappedAccountId(contract)));
        }
        env::log_str(&format!("Whitelist updated with {} contracts by {}", self.whitelisted_contracts.len(), caller.as_str()));
        Ok(())
    }

    pub fn set_sponsor_amount(&mut self, amount: U128) -> Result<(), RelayerError> {
        let caller = env::predecessor_account_id();
        if !self.admins.iter().any(|admin| admin.0.0 == caller) {
            return Err(RelayerError::Unauthorized);
        }
        if amount.0 < 50_000_000_000_000_000_000_000 {
            return Err(RelayerError::InvalidSponsorAmount);
        }
        self.sponsor_amount = amount.0;
        env::log_str(&format!("Sponsor amount updated to {} yoctoNEAR by {}", amount.0, caller.as_str()));
        Ok(())
    }

    pub fn set_admins(&mut self, new_admins: Vec<AccountId>) -> Result<(), RelayerError> {
        let caller = env::predecessor_account_id();
        if !self.admins.iter().any(|admin| admin.0.0 == caller) {
            return Err(RelayerError::Unauthorized);
        }
        if new_admins.is_empty() {
            return Err(RelayerError::InvalidAccountId);
        }
        self.admins.clear();
        for admin in new_admins {
            self.admins.push(AccountIdWrapper::from(WrappedAccountId(admin)));
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
        if !self.admins.iter().any(|admin| admin.0.0 == caller) {
            return Err(RelayerError::Unauthorized);
        }
        let action = Action::AddKey {
            public_key: WrappedPublicKey(public_key),
            allowance: Some(WrappedNearToken(NearToken::from_yoctonear(100_000_000_000_000_000_000_000))),
            receiver_id: WrappedAccountId(receiver_id.clone()),
            method_names,
        };
        let delegate = DelegateAction {
            sender_id: WrappedAccountId(account_id.clone()),
            receiver_id: WrappedAccountId(receiver_id),
            actions: vec![action],
            nonce: *self.processed_nonces.get(&AccountIdWrapper::from(WrappedAccountId(account_id.clone()))).unwrap_or(&0) + 1,
            max_block_height: env::block_height() + self.default_max_block_height_delta,
        };
        let mut pk_bytes = vec![0];
        pk_bytes.extend_from_slice(&[0; 32]);
        let dummy_pk = PublicKey::try_from(pk_bytes).unwrap();
        let signed_delegate = SignedDelegateAction {
            delegate_action: delegate,
            signature: vec![0; 64],
            public_key: WrappedPublicKey(dummy_pk),
        };
        self.relay_meta_transaction(signed_delegate)
    }

    pub fn remove_function_call_key(
        &mut self,
        account_id: AccountId,
        public_key: PublicKey,
        signed_delegate: SignedDelegateAction,
    ) -> Result<Promise, RelayerError> {
        require!(env::attached_deposit().as_yoctonear() == 0, "No deposit allowed; costs covered by relayer");

        let delegate = &signed_delegate.delegate_action;

        require!(
            delegate.sender_id.0 == account_id &&
            delegate.receiver_id.0 == env::current_account_id() &&
            delegate.actions.len() == 1,
            "Invalid delegate action"
        );

        if let Action::FunctionCall { method_name, args, deposit, .. } = &delegate.actions[0] {
            require!(method_name == "remove_function_call_key", "Must call remove_function_call_key");
            require!(deposit.0.as_yoctonear() == 0, "No deposit allowed in delegate action");

            let params: serde_json::Value = serde_json::from_slice(args).unwrap();
            require!(
                params["account_id"] == account_id.to_string() &&
                params["public_key"] == serde_json::to_value(&WrappedPublicKey(public_key.clone())).unwrap(),
                "Delegate args mismatch"
            );
        } else {
            return Err(RelayerError::InvalidKeyAction);
        }

        #[cfg(not(test))]
        {
            use ed25519_dalek::{VerifyingKey, Signature, Verifier};
            let message = near_sdk::borsh::to_vec(&delegate).map_err(|_| RelayerError::InvalidSignature)?;
            let public_key_bytes = &signed_delegate.public_key.0.as_bytes()[1..];
            let verifying_key = VerifyingKey::from_bytes(
                public_key_bytes.try_into().map_err(|_| RelayerError::InvalidSignature)?
            ).map_err(|_| RelayerError::InvalidSignature)?;
            let signature = Signature::from_bytes(
                &signed_delegate.signature.try_into().map_err(|_| RelayerError::InvalidSignature)?
            );
            verifying_key.verify(&message, &signature).map_err(|_| RelayerError::InvalidSignature)?;
        }
        #[cfg(test)]
        {
            if self.simulate_signature_failure {
                return Err(RelayerError::InvalidSignature);
            }
            env::log_str("Signature verification simulated in test");
        }

        let required_balance = self.min_gas_pool.checked_add((self.default_gas / 1_000_000_000_000).into()).unwrap();
        if env::account_balance().as_yoctonear() < required_balance {
            return Err(RelayerError::InsufficientGasPool);
        }

        let last_nonce = self.processed_nonces.get(&AccountIdWrapper::from(WrappedAccountId(account_id.clone()))).unwrap_or(&0);
        if delegate.nonce <= *last_nonce {
            return Err(RelayerError::InvalidNonce);
        }

        if env::block_height() > delegate.max_block_height {
            return Err(RelayerError::ExpiredTransaction);
        }

        self.processed_nonces.insert(AccountIdWrapper::from(WrappedAccountId(account_id.clone())), delegate.nonce);

        RelayerEvent::FunctionCallKeyRemoved { account_id: account_id.clone(), public_key: public_key.clone() }.emit();

        Ok(Promise::new(account_id).delete_key(public_key))
    }

    pub fn retry_or_clear_failed_transactions(&mut self, retry: bool) -> Result<(), RelayerError> {
        let caller = env::predecessor_account_id();
        if !self.admins.iter().any(|admin| admin.0.0 == caller) {
            return Err(RelayerError::Unauthorized);
        }
        if self.failed_transactions.is_empty() {
            return Err(RelayerError::NoFailedTransactions);
        }
    
        self.clean_failed_transactions();
        let count = self.failed_transactions.len() as u32;
    
        if retry {
            let mut failed = Vec::new();
            std::mem::swap(&mut self.failed_transactions, &mut failed);
            let mut retry_count = 0;
            for (signed_delegate, gas, _reason) in failed {
                let new_gas = (gas as u64).saturating_mul(12) / 10;
                let new_gas_with_buffer = new_gas.saturating_add(self.gas_buffer);
                let new_gas = new_gas_with_buffer.min(300_000_000_000_000);
                match self.relay_meta_transaction(signed_delegate.clone()) {
                    Ok(()) => {
                        retry_count += 1;
                        env::log_str(&format!("Manual retry of transaction with nonce {} succeeded", signed_delegate.delegate_action.nonce));
                    }
                    Err(e) => {
                        if self.failed_transactions.len() < crate::state::FAILED_TX_QUEUE_CAP as usize {
                            self.failed_transactions.push((signed_delegate.clone(), new_gas, Some(e.clone())));
                            env::log_str(&format!("Manual retry failed: {:?}", e));
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
        self.admins.iter().map(|wrapper| wrapper.0.0.clone()).collect()
    }

    pub fn get_default_gas(&self) -> Gas {
        Gas::from_gas(self.default_gas)
    }

    pub fn get_gas_buffer(&self) -> Gas {
        Gas::from_gas(self.gas_buffer)
    }

    pub fn get_failed_transactions_count(&self) -> u32 {
        self.failed_transactions.len() as u32
    }

    pub fn get_failed_transactions(&self) -> Vec<(SignedDelegateAction, u64, Option<RelayerError>)> {
        self.failed_transactions.clone()
    }

    pub fn get_failed_transactions_by_sender(&self, sender_id: AccountId) -> Vec<(SignedDelegateAction, u64, Option<RelayerError>)> {
        self.failed_transactions
            .iter()
            .filter(|(signed_delegate, _, _)| signed_delegate.delegate_action.sender_id.0 == sender_id)
            .cloned()
            .collect()
    }

    pub fn get_processed_nonce(&self, account_id: AccountId) -> Option<u64> {
        self.processed_nonces.get(&AccountIdWrapper::from(WrappedAccountId(account_id))).copied()
    }

    pub fn set_max_block_height_delta(&mut self, delta: u64) -> Result<(), RelayerError> {
        let caller = env::predecessor_account_id();
        if !self.admins.iter().any(|admin| admin.0.0 == caller) {
            return Err(RelayerError::Unauthorized);
        }
        if delta < 100 || delta > 10_000 {
            return Err(RelayerError::InvalidGasConfig);
        }
        self.default_max_block_height_delta = delta;
        env::log_str(&format!(
            "Max block height delta set to {} blocks (~{} minutes) by {}",
            delta, delta / 60, caller.as_str()
        ));
        Ok(())
    }

    pub fn get_max_block_height_delta(&self) -> u64 {
        self.default_max_block_height_delta
    }

    // New method to check pending transaction status
    pub fn get_pending_transaction(&self, sender_id: AccountId, nonce: u64) -> Option<(u64, bool)> {
        // Check if it's already processed
        if let Some(&processed_nonce) = self.processed_nonces.get(&AccountIdWrapper::from(WrappedAccountId(sender_id.clone()))) {
            if nonce <= processed_nonce {
                return None; // Already done or too old
            }
        }
        // Check if it's in the failed queue (pending retry)
        for (signed_delegate, _, _) in &self.failed_transactions {
            if signed_delegate.delegate_action.sender_id.0 == sender_id && signed_delegate.delegate_action.nonce == nonce {
                let is_expired = env::block_height() > signed_delegate.delegate_action.max_block_height;
                return Some((signed_delegate.delegate_action.max_block_height, is_expired));
            }
        }
        // Not found in processed or failed—assume pending elsewhere (e.g., relayer queue) if nonce is next
        let next_nonce = self.processed_nonces.get(&AccountIdWrapper::from(WrappedAccountId(sender_id.clone()))).map_or(0, |&n| n + 1);
        if nonce == next_nonce {
            let default_expiry = env::block_height() + self.default_max_block_height_delta;
            return Some((default_expiry, false)); // Assume pending, not expired yet
        }
        None // Not pending or known
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