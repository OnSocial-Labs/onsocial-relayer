use near_sdk::{Promise, Allowance, AccountId, NearToken, Gas, env, ext_contract};
use serde_json::json;
use crate::state::Relayer;
use crate::types::{SignedDelegateAction, Action};
use crate::errors::RelayerError;
use crate::events::RelayerEvent;
use core::num::NonZeroU128;

const MAX_GAS: Gas = Gas::from_tgas(290);
const CHUNK_SIZE: usize = 5;

#[ext_contract(ext_self)]
#[allow(dead_code)]
trait SelfCallback {
    fn refund_gas_callback(&mut self, initial_cost: u128);
}

pub fn relay_meta_transaction(relayer: &mut Relayer, signed_delegate: SignedDelegateAction) -> Result<Promise, RelayerError> {
    let sender_id = &signed_delegate.delegate_action.sender_id;
    match relayer.auth_accounts.get(sender_id) {
        Some(auth_key) if *auth_key == signed_delegate.public_key => (), // Dereference auth_key
        _ => return Err(RelayerError::Unauthorized),
    }

    let max_gas_cost = 29_000_000_000_000_000_000_000;
    if relayer.gas_pool < relayer.min_gas_pool + max_gas_cost {
        RelayerEvent::LowGasPool { remaining: relayer.gas_pool }.emit();
        return Err(RelayerError::InsufficientGasPool);
    }
    
    relayer.gas_pool -= max_gas_cost;
    let delegate = signed_delegate.delegate_action;
    let mut promise = Promise::new(delegate.sender_id.clone());
    for action in delegate.actions {
        match action {
            Action::FunctionCall { method_name, args, gas, deposit } => {
                let capped_gas = if gas > MAX_GAS { MAX_GAS } else { gas };
                promise = promise.function_call(method_name, args, deposit, capped_gas);
            }
            Action::Transfer { deposit } => {
                promise = promise.transfer(deposit);
            }
            Action::AddKey { public_key, allowance, receiver_id, method_names } => {
                promise = promise.add_access_key_allowance(
                    public_key,
                    allowance.map_or(Allowance::Unlimited, |t| Allowance::Limited(NonZeroU128::new(t.as_yoctonear()).unwrap())),
                    receiver_id,
                    method_names.join(",")
                );
            }
            Action::ChainSignatureRequest { target_chain, derivation_path, payload } => {
                let mpc_contract: AccountId = target_chain.split('|').last().unwrap_or(&target_chain).parse()
                    .map_err(|_| RelayerError::InvalidAccountId)?;
                let args = serde_json::to_vec(&json!({"request": {"payload": payload, "path": derivation_path, "key_version": 0}}))
                    .map_err(|_| RelayerError::InvalidAccountId)?;
                promise = Promise::new(mpc_contract)
                    .function_call("sign".to_string(), args, NearToken::from_yoctonear(1), MAX_GAS);
            }
        }
    }

    let callback = ext_self::ext(env::current_account_id())
        .with_static_gas(Gas::from_tgas(5))
        .refund_gas_callback(max_gas_cost);
    Ok(promise.then(callback))
}

pub fn relay_meta_transactions(relayer: &mut Relayer, signed_delegates: Vec<SignedDelegateAction>) -> Result<Vec<Promise>, RelayerError> {
    if signed_delegates.is_empty() || signed_delegates.len() > CHUNK_SIZE {
        return Err(RelayerError::InvalidNonce);
    }

    let total_gas_cost = 29_000_000_000_000_000_000_000 * signed_delegates.len() as u128;
    if relayer.gas_pool < relayer.min_gas_pool + total_gas_cost {
        RelayerEvent::LowGasPool { remaining: relayer.gas_pool }.emit();
        return Err(RelayerError::InsufficientGasPool);
    }

    let promises: Vec<Promise> = signed_delegates.into_iter()
        .map(|signed_delegate| {
            let sender_id = &signed_delegate.delegate_action.sender_id;
            match relayer.auth_accounts.get(sender_id) {
                Some(auth_key) if *auth_key == signed_delegate.public_key => (), // Dereference auth_key
                _ => return Promise::new(env::current_account_id()),
            }

            let mut promise = Promise::new(signed_delegate.delegate_action.sender_id.clone());
            for action in signed_delegate.delegate_action.actions {
                match action {
                    Action::FunctionCall { method_name, args, gas, deposit } => {
                        let capped_gas = if gas > MAX_GAS { MAX_GAS } else { gas };
                        promise = promise.function_call(method_name, args, deposit, capped_gas);
                    }
                    Action::Transfer { deposit } => {
                        promise = promise.transfer(deposit);
                    }
                    Action::AddKey { public_key, allowance, receiver_id, method_names } => {
                        promise = promise.add_access_key_allowance(
                            public_key,
                            allowance.map_or(Allowance::Unlimited, |t| Allowance::Limited(NonZeroU128::new(t.as_yoctonear()).unwrap())),
                            receiver_id,
                            method_names.join(",")
                        );
                    }
                    Action::ChainSignatureRequest { target_chain, derivation_path, payload } => {
                        let mpc_contract = match target_chain.split('|').last().unwrap_or(&target_chain).parse() {
                            Ok(account) => account,
                            Err(_) => return Promise::new(env::current_account_id()),
                        };
                        let args = match serde_json::to_vec(&json!({"request": {"payload": payload, "path": derivation_path, "key_version": 0}})) {
                            Ok(args) => args,
                            Err(_) => return Promise::new(env::current_account_id()),
                        };
                        promise = Promise::new(mpc_contract)
                            .function_call("sign".to_string(), args, NearToken::from_yoctonear(1), MAX_GAS);
                    }
                }
            }
            relayer.gas_pool -= 29_000_000_000_000_000_000_000;
            let callback = ext_self::ext(env::current_account_id())
                .with_static_gas(Gas::from_tgas(5))
                .refund_gas_callback(29_000_000_000_000_000_000_000);
            promise.then(callback)
        })
        .collect();

    Ok(promises)
}

pub fn relay_chunked_meta_transactions(relayer: &mut Relayer, signed_delegates: Vec<SignedDelegateAction>) -> Result<Vec<Promise>, RelayerError> {
    if signed_delegates.is_empty() {
        return Err(RelayerError::InvalidNonce);
    }

    let total_gas_cost = 29_000_000_000_000_000_000_000 * signed_delegates.len() as u128;
    if relayer.gas_pool < relayer.min_gas_pool + total_gas_cost {
        RelayerEvent::LowGasPool { remaining: relayer.gas_pool }.emit();
        return Err(RelayerError::InsufficientGasPool);
    }

    let mut all_promises = Vec::new();
    for chunk in signed_delegates.chunks(CHUNK_SIZE) {
        let chunk_promises: Vec<Promise> = chunk.iter()
            .map(|signed_delegate| {
                let sender_id = &signed_delegate.delegate_action.sender_id;
                match relayer.auth_accounts.get(sender_id) {
                    Some(auth_key) if *auth_key == signed_delegate.public_key => (), // Dereference auth_key
                    _ => return Promise::new(env::current_account_id()),
                }

                let mut promise = Promise::new(signed_delegate.delegate_action.sender_id.clone());
                for action in signed_delegate.delegate_action.actions.iter() {
                    match action {
                        Action::FunctionCall { method_name, args, gas, deposit } => {
                            let capped_gas = if *gas > MAX_GAS { MAX_GAS } else { *gas };
                            promise = promise.function_call(method_name.clone(), args.clone(), *deposit, capped_gas);
                        }
                        Action::Transfer { deposit } => {
                            promise = promise.transfer(*deposit);
                        }
                        Action::AddKey { public_key, allowance, receiver_id, method_names } => {
                            promise = promise.add_access_key_allowance(
                                public_key.clone(),
                                allowance.map_or(Allowance::Unlimited, |t| Allowance::Limited(NonZeroU128::new(t.as_yoctonear()).unwrap())),
                                receiver_id.clone(),
                                method_names.join(",")
                            );
                        }
                        Action::ChainSignatureRequest { target_chain, derivation_path, payload } => {
                            let mpc_contract = match target_chain.split('|').last().unwrap_or(target_chain).parse() {
                                Ok(account) => account,
                                Err(_) => return Promise::new(env::current_account_id()),
                            };
                            let args = match serde_json::to_vec(&json!({"request": {"payload": payload.clone(), "path": derivation_path.clone(), "key_version": 0}})) {
                                Ok(args) => args,
                                Err(_) => return Promise::new(env::current_account_id()),
                            };
                            promise = Promise::new(mpc_contract)
                                .function_call("sign".to_string(), args, NearToken::from_yoctonear(1), MAX_GAS);
                        }
                    }
                }
                relayer.gas_pool -= 29_000_000_000_000_000_000_000;
                let callback = ext_self::ext(env::current_account_id())
                    .with_static_gas(Gas::from_tgas(5))
                    .refund_gas_callback(29_000_000_000_000_000_000_000);
                promise.then(callback)
            })
            .collect();
        all_promises.extend(chunk_promises);
    }

    Ok(all_promises)
}