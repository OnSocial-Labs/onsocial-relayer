use near_sdk::{env, AccountId, Promise, PublicKey, NearToken, Gas, require};
use near_sdk::json_types::U128;
use serde_json;
use core::num::NonZeroU128;
use crate::state::{Relayer, AccountIdWrapper};
use crate::errors::RelayerError;
use crate::events::RelayerEvent;
use crate::types::{WrappedAccountId, SignedDelegateAction, Action};

impl Relayer {
    pub fn sponsor_account(
        &mut self,
        account_name: String,
        public_key: PublicKey,
        add_function_call_key: bool,
        is_implicit: bool,
        signed_delegate: Option<SignedDelegateAction>,
    ) -> Result<Promise, RelayerError> {
        let _sender_id = if let Some(signed_delegate) = &signed_delegate {
            require!(env::attached_deposit().as_yoctonear() == 0, "No deposit allowed");
            let delegate = &signed_delegate.delegate_action;
            require!(
                delegate.receiver_id.0 == env::current_account_id() &&
                delegate.actions.len() == 1 &&
                matches!(&delegate.actions[0], Action::FunctionCall { method_name, deposit, .. } if method_name == "sponsor_account" && deposit.0.as_yoctonear() == 0),
                "Invalid delegate action"
            );
            #[cfg(not(test))]
            {
                use ed25519_dalek::{VerifyingKey, Signature, Verifier};
                let message = near_sdk::borsh::to_vec(&delegate).map_err(|_| RelayerError::InvalidSignature)?;
                let public_key_bytes = &signed_delegate.public_key.0.as_bytes()[1..];
                let verifying_key = VerifyingKey::from_bytes(
                    public_key_bytes.try_into().map_err(|_| RelayerError::InvalidSignature)?
                ).map_err(|_| RelayerError::InvalidSignature)?;
                let signature = Signature::from_bytes(
                    &signed_delegate.signature.clone().try_into().map_err(|_| RelayerError::InvalidSignature)?
                );
                verifying_key.verify(&message, &signature).map_err(|_| RelayerError::InvalidSignature)?;
            }
            Some(delegate.sender_id.0.clone())
        } else {
            require!(env::predecessor_account_id() == env::current_account_id(), "Direct call not allowed");
            None
        };

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

        let account_id_wrapper = AccountIdWrapper::from(WrappedAccountId(new_account_id.clone()));
        if self.sponsored_accounts.contains_key(&account_id_wrapper) {
            return Err(RelayerError::AccountExists);
        }

        let gas_cost = if add_function_call_key { (self.default_gas / 1_000_000_000_000).into() } else { 0 };
        let required_balance = self.min_gas_pool
            .checked_add(self.sponsor_amount)
            .and_then(|sum| sum.checked_add(gas_cost))
            .ok_or(RelayerError::InsufficientBalance)?;
        if env::account_balance().as_yoctonear() < required_balance {
            return Err(RelayerError::InsufficientBalance);
        }

        let mut promise = if is_implicit {
            Promise::new(new_account_id.clone()).transfer(NearToken::from_yoctonear(self.sponsor_amount))
        } else {
            let root_account = if env::current_account_id().as_str().contains(".testnet") { "testnet" } else { "near" }
                .parse()
                .map_err(|_| RelayerError::InvalidAccountId)?;
            let args = serde_json::json!({
                "new_account_id": new_account_id.as_str(),
                "new_public_key": public_key
            });
            let serialized_args = serde_json::to_vec(&args)
                .map_err(|_| RelayerError::InvalidAccountId)?;
            Promise::new(root_account).function_call(
                "create_account".to_string(),
                serialized_args,
                NearToken::from_yoctonear(self.sponsor_amount),
                Gas::from_gas(self.default_gas),
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

        RelayerEvent::AccountSponsored { account_id: new_account_id.clone(), public_key, is_implicit }.emit();

        self.sponsored_accounts.insert(account_id_wrapper, true);

        if let Some(signed_delegate) = signed_delegate {
            RelayerEvent::MetaTransactionRelayed {
                sender_id: signed_delegate.delegate_action.sender_id.0.clone(),
                nonce: signed_delegate.delegate_action.nonce,
            }.emit();
        }

        Ok(promise)
    }

    pub fn get_sponsor_amount(&self) -> U128 {
        U128(self.sponsor_amount)
    }
}