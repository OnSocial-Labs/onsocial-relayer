use near_sdk::{env, AccountId, Gas};
use serde_json;
use crate::state::{Relayer, AccountIdWrapper};
use crate::types::{SignedDelegateAction, WrappedAccountId};
use crate::errors::RelayerError;
use crate::events::RelayerEvent;

impl Relayer {
    pub fn relay_meta_transaction(&mut self, signed_delegate: SignedDelegateAction) -> Result<(), RelayerError> {
        self.clean_failed_transactions();

        let delegate = &signed_delegate.delegate_action;
        let sender = &delegate.sender_id;

        if env::account_balance().as_yoctonear() < self.min_gas_pool {
            return Err(RelayerError::InsufficientGasPool);
        }

        let last_nonce = self.processed_nonces.get(&AccountIdWrapper::from(sender.clone())).unwrap_or(&0);
        if delegate.nonce <= *last_nonce {
            return Err(RelayerError::InvalidNonce);
        }

        if env::block_height() > delegate.max_block_height {
            return Err(RelayerError::ExpiredTransaction);
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
        RelayerEvent::MetaTransactionRelayed { sender_id: delegate.sender_id.0.clone(), nonce: delegate.nonce }.emit();
        Ok(())
    }

    pub fn callback_success(&mut self, sender_id: AccountId, nonce: u64) {
        self.processed_nonces.insert(AccountIdWrapper::from(WrappedAccountId(sender_id.clone())), nonce);
        RelayerEvent::MetaTransactionRelayed { sender_id, nonce }.emit();
    }

    pub fn callback_failure(&mut self, signed_delegate: SignedDelegateAction, gas: Gas) {
        self.clean_failed_transactions();
    
        #[cfg(test)]
        let promise_result = self.simulate_promise_result
            .clone()
            .unwrap_or(crate::types::SerializablePromiseResult::Failed);
        #[cfg(not(test))]
        let promise_result = crate::types::SerializablePromiseResult::from(env::promise_result(0));
    
        if let crate::types::SerializablePromiseResult::Failed = promise_result {
            let new_gas = (gas.as_gas() as u64).saturating_mul(12) / 10; // 120% of original
            let new_gas_with_buffer = new_gas.saturating_add(self.gas_buffer); // Add buffer
            let new_gas = new_gas_with_buffer.min(300_000_000_000_000); // Cap at 300 TGas
            if env::account_balance().as_yoctonear() >= self.min_gas_pool && 
               env::block_height() <= signed_delegate.delegate_action.max_block_height {
                match self.relay_meta_transaction(signed_delegate.clone()) {
                    Ok(()) => env::log_str(&format!("Auto-retried transaction with nonce {} succeeded", signed_delegate.delegate_action.nonce)),
                    Err(_e) => {
                        if self.failed_transactions.len() < crate::state::FAILED_TX_QUEUE_CAP as usize {
                            self.failed_transactions.push((signed_delegate.clone(), new_gas));
                            env::log_str(&format!("Queued failed transaction with {} TGas", new_gas / 1_000_000_000_000));
                        } else {
                            env::log_str(&format!(
                                "Failed transaction with nonce {} dropped due to queue cap",
                                signed_delegate.delegate_action.nonce
                            ));
                        }
                    }
                }
            } else if self.failed_transactions.len() < crate::state::FAILED_TX_QUEUE_CAP as usize {
                self.failed_transactions.push((signed_delegate, new_gas));
                env::log_str(&format!("Queued failed transaction with {} TGas", new_gas / 1_000_000_000_000));
            } else {
                env::log_str(&format!(
                    "Failed transaction with nonce {} dropped due to queue cap",
                    signed_delegate.delegate_action.nonce
                ));
            }
        }
    }

    pub fn callback_key_addition(&mut self, sender_id: AccountId) {
        #[cfg(test)]
        let promise_result = self.simulate_promise_result
            .clone()
            .unwrap_or(crate::types::SerializablePromiseResult::Successful(vec![]));
        #[cfg(not(test))]
        let promise_result = crate::types::SerializablePromiseResult::from(env::promise_result(0));

        if let crate::types::SerializablePromiseResult::Successful(_) = promise_result {
            RelayerEvent::FunctionCallKeyAdded {
                account_id: sender_id.clone(),
                public_key: env::signer_account_pk(),
                receiver_id: env::current_account_id(),
            }.emit();
        }
    }
}