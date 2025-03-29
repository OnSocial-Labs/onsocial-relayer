use near_sdk::{env, AccountId, Gas, require, PublicKey, Promise};
use serde_json;
use std::num::NonZeroU128;
use crate::state::{Relayer, AccountIdWrapper};
use crate::types::{SignedDelegateAction, WrappedAccountId, Action, WrappedPublicKey, SerializablePromiseResult};
use crate::errors::RelayerError;
use crate::events::RelayerEvent;

impl Relayer {
    pub fn relay_meta_transaction(&mut self, signed_delegate: SignedDelegateAction) -> Result<(), RelayerError> {
        self.clean_failed_transactions();

        let delegate = &signed_delegate.delegate_action;
        let sender = &delegate.sender_id;

        require!(env::attached_deposit().as_yoctonear() == 0, "No deposit allowed");

        if env::account_balance().as_yoctonear() < self.min_gas_pool {
            return Err(RelayerError::InsufficientGasPool);
        }

        let last_nonce = self.processed_nonces.get(&AccountIdWrapper::from(sender.clone())).unwrap_or(&0);
        if delegate.nonce <= *last_nonce {
            return Err(RelayerError::InvalidNonce);
        }

        if env::block_height() > delegate.max_block_height {
            env::log_str(&format!(
                "Transaction expired: current block {}, max block {}",
                env::block_height(), delegate.max_block_height
            ));
            return Err(RelayerError::ExpiredTransaction);
        }

        if delegate.receiver_id.0 == env::current_account_id() {
            if let Action::FunctionCall { method_name, args, deposit, .. } = &delegate.actions[0] {
                require!(deposit.0.as_yoctonear() == 0, "No deposit allowed in delegate action");
                match method_name.as_str() {
                    "import_account" => {
                        let params: serde_json::Value = serde_json::from_slice(args).unwrap();
                        let account_id: AccountId = params["account_id"].as_str().unwrap().parse().unwrap();
                        let public_key: PublicKey = serde_json::from_value(params["public_key"].clone()).unwrap();
                        self.import_account(account_id, public_key, signed_delegate.clone())?;
                        return Ok(());
                    }
                    "sponsor_account" => {
                        let params: serde_json::Value = serde_json::from_slice(args).unwrap();
                        let account_name: String = params["account_name"].as_str().unwrap().to_string();
                        let public_key: PublicKey = serde_json::from_value(params["public_key"].clone()).unwrap();
                        let add_function_call_key: bool = params["add_function_call_key"].as_bool().unwrap();
                        let is_implicit: bool = params["is_implicit"].as_bool().unwrap();
                        self.sponsor_account(account_name, public_key, add_function_call_key, is_implicit, Some(signed_delegate.clone()))?;
                        return Ok(());
                    }
                    _ => return Err(RelayerError::InvalidKeyAction),
                }
            }
        }

        if !self.whitelisted_contracts.iter().any(|id| id.0 == delegate.receiver_id) {
            return Err(RelayerError::NotWhitelisted);
        }

        #[cfg(not(test))]
        {
            use ed25519_dalek::{VerifyingKey, Signature, Verifier};
            let message = near_sdk::borsh::to_vec(&delegate).map_err(|_| RelayerError::InvalidSignature)?;
            if signed_delegate.signature.len() != 64 {
                return Err(RelayerError::InvalidSignature);
            }
            let public_key_bytes = &signed_delegate.public_key.0.as_bytes()[1..];
            let public_key = VerifyingKey::from_bytes(
                public_key_bytes.try_into().map_err(|_| RelayerError::InvalidSignature)?
            ).map_err(|_| RelayerError::InvalidSignature)?;
            let signature_bytes: [u8; 64] = signed_delegate.signature.try_into()
                .map_err(|_| RelayerError::InvalidSignature)?;
            let signature = Signature::from_bytes(&signature_bytes);
            if public_key.verify(&message, &signature).is_err() {
                return Err(RelayerError::InvalidSignature);
            }
        }
        #[cfg(test)]
        {
            if self.simulate_signature_failure {
                return Err(RelayerError::InvalidSignature);
            }
            env::log_str("Signature verification simulated in test");
        }

        if let Some(ft_contract) = &self.payment_ft_contract {
            if delegate.actions.is_empty() {
                return Err(RelayerError::NoActions);
            }
            if let crate::types::Action::FunctionCall { method_name, args, deposit, .. } = &delegate.actions[0] {
                if method_name != "ft_transfer" {
                    return Err(RelayerError::InvalidFTTransfer);
                }
                let args: serde_json::Value = serde_json::from_slice(args).unwrap();
                if args["receiver_id"].as_str().unwrap() != env::current_account_id().as_str() {
                    return Err(RelayerError::InvalidFTTransfer);
                }
                if deposit.0.as_yoctonear() < self.min_ft_payment {
                    return Err(RelayerError::InsufficientDeposit);
                }
                if ft_contract.0 != delegate.receiver_id {
                    return Err(RelayerError::InvalidFTTransfer);
                }
            } else {
                return Err(RelayerError::InvalidFTTransfer);
            }
        }

        self.processed_nonces.insert(AccountIdWrapper::from(sender.clone()), delegate.nonce);

        env::log_str(&format!(
            "Transaction relayed for {} with nonce {}. Expires at block {} (~{} minutes from now)",
            delegate.sender_id.0.as_str(),
            delegate.nonce,
            delegate.max_block_height,
            (delegate.max_block_height - env::block_height()) / 60
        ));
        RelayerEvent::MetaTransactionRelayed { sender_id: delegate.sender_id.0.clone(), nonce: delegate.nonce }.emit();
        Ok(())
    }

    pub fn import_account(
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
            require!(method_name == "import_account", "Must call import_account");
            require!(deposit.0.as_yoctonear() == 0, "No deposit allowed in delegate action");
            let args: serde_json::Value = serde_json::from_slice(args).unwrap();
            require!(
                args["account_id"] == account_id.to_string() && 
                args["public_key"] == serde_json::to_value(&WrappedPublicKey(public_key.clone())).unwrap(),
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

        let required_balance = self.min_gas_pool.checked_add((self.default_gas / 1_000_000_000_000).into()).unwrap();
        if env::account_balance().as_yoctonear() < required_balance {
            return Err(RelayerError::InsufficientGasPool);
        }

        let promise = Promise::new(account_id.clone()).add_access_key_allowance(
            public_key.clone(),
            near_sdk::Allowance::Limited(NonZeroU128::new(100_000_000_000_000_000_000_000).unwrap()),
            env::current_account_id(),
            "relay_meta_transaction".to_string(),
        );

        RelayerEvent::FunctionCallKeyAdded {
            account_id,
            public_key,
            receiver_id: env::current_account_id(),
        }.emit();

        Ok(promise)
    }

    pub fn callback_success(&mut self, sender_id: AccountId, nonce: u64) {
        let sender_wrapper = AccountIdWrapper::from(WrappedAccountId(sender_id.clone()));
        if self.processed_nonces.get(&sender_wrapper).map_or(true, |&existing_nonce| nonce > existing_nonce) {
            self.processed_nonces.insert(sender_wrapper, nonce);
            RelayerEvent::MetaTransactionRelayed { sender_id, nonce }.emit();
        }
    }

    pub fn callback_failure(&mut self, signed_delegate: SignedDelegateAction, gas: Gas) {
        self.clean_failed_transactions();
    
        #[cfg(test)]
        let promise_result = self.simulate_promise_result
            .clone()
            .unwrap_or(SerializablePromiseResult::Failed);
        #[cfg(not(test))]
        let promise_result = SerializablePromiseResult::from(env::promise_result(0));
    
        if let SerializablePromiseResult::Failed = promise_result {
            let new_gas = (gas.as_gas() as u64).saturating_mul(12) / 10;
            let new_gas_with_buffer = new_gas.saturating_add(self.gas_buffer);
            let new_gas = new_gas_with_buffer.min(300_000_000_000_000);
            if env::account_balance().as_yoctonear() >= self.min_gas_pool && 
               env::block_height() <= signed_delegate.delegate_action.max_block_height {
                match self.relay_meta_transaction(signed_delegate.clone()) {
                    Ok(()) => {
                        env::log_str(&format!("Auto-retried transaction with nonce {} succeeded", signed_delegate.delegate_action.nonce));
                    }
                    Err(e) => {
                        if self.failed_transactions.len() < crate::state::FAILED_TX_QUEUE_CAP as usize {
                            self.failed_transactions.push((signed_delegate.clone(), new_gas, Some(e.clone())));
                            env::log_str(&format!("Queued failed transaction with {} TGas, reason: {:?}", new_gas / 1_000_000_000_000, e));
                        } else {
                            env::log_str(&format!(
                                "Failed transaction with nonce {} dropped due to queue cap",
                                signed_delegate.delegate_action.nonce
                            ));
                        }
                    }
                }
            } else {
                let error_reason = if env::account_balance().as_yoctonear() < self.min_gas_pool {
                    Some(RelayerError::InsufficientGasPool)
                } else if env::block_height() > signed_delegate.delegate_action.max_block_height {
                    Some(RelayerError::ExpiredTransaction)
                } else {
                    None
                };
                if self.failed_transactions.len() < crate::state::FAILED_TX_QUEUE_CAP as usize {
                    self.failed_transactions.push((signed_delegate, new_gas, error_reason.clone()));
                    env::log_str(&format!("Queued failed transaction with {} TGas, reason: {:?}", new_gas / 1_000_000_000_000, error_reason));
                } else {
                    env::log_str(&format!(
                        "Failed transaction with nonce {} dropped due to queue cap",
                        signed_delegate.delegate_action.nonce
                    ));
                }
            }
        }
    }

    pub fn callback_key_addition(&mut self, sender_id: AccountId) {
        #[cfg(test)]
        let promise_result = self.simulate_promise_result
            .clone()
            .unwrap_or(SerializablePromiseResult::Successful(vec![]));
        #[cfg(not(test))]
        let promise_result = SerializablePromiseResult::from(env::promise_result(0));

        if let SerializablePromiseResult::Successful(_) = promise_result {
            RelayerEvent::FunctionCallKeyAdded {
                account_id: sender_id.clone(),
                public_key: env::signer_account_pk(),
                receiver_id: env::current_account_id(),
            }.emit();
        }
    }
}