use near_sdk::{Promise, Allowance, AccountId, NearToken, Gas, env, ext_contract};
use near_sdk::borsh::{self, BorshSerialize, BorshDeserialize};
use core::num::NonZeroU128;
use crate::state::Relayer;
use crate::types::{SignedDelegateAction, Action};
use crate::errors::RelayerError;
use crate::events::RelayerEvent;

#[derive(BorshSerialize, BorshDeserialize)]
struct SignRequest {
    payload: Vec<u8>,
    path: String,
    key_version: u32,
}

const MAX_GAS: Gas = Gas::from_tgas(290);
const MPC_SIGN_GAS: Gas = Gas::from_tgas(100); // Adjusted for MPC sign call; profile on testnet
const CHUNK_SIZE: usize = 5;

#[ext_contract(ext_self)]
#[allow(dead_code)] // Suppress warning since it's used by ext_contract macro
trait SelfCallback {
    fn refund_gas_callback(&mut self, initial_cost: u128);
}

pub fn relay_meta_transaction(relayer: &mut Relayer, signed_delegate: SignedDelegateAction) -> Result<Promise, RelayerError> {
    let sender_id = &signed_delegate.delegate_action.sender_id;
    match relayer.auth_accounts.get(sender_id) {
        Some(auth_key) if *auth_key == signed_delegate.public_key => (),
        _ => return Err(RelayerError::Unauthorized),
    }

    let max_gas_cost = 29_000_000_000_000_000_000_000_u128;
    if relayer.gas_pool < relayer.min_gas_pool + max_gas_cost {
        RelayerEvent::LowGasPool { remaining: relayer.gas_pool }.emit();
        return Err(RelayerError::InsufficientGasPool);
    }

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
                let mpc_contract: AccountId = match target_chain.parse() {
                    Ok(account) => account,
                    Err(_) => return Err(RelayerError::InvalidAccountId),
                };
                let request = SignRequest {
                    payload,
                    path: derivation_path,
                    key_version: 0,
                };
                let args = match borsh::to_vec(&request) {
                    Ok(args) => args,
                    Err(_) => return Err(RelayerError::InvalidAccountId),
                };
                promise = Promise::new(mpc_contract)
                    .function_call("sign".to_string(), args, NearToken::from_yoctonear(1), MPC_SIGN_GAS);
            }
        }
    }

    relayer.gas_pool -= max_gas_cost;
    let callback = ext_self::ext(env::current_account_id())
        .with_static_gas(Gas::from_tgas(5))
        .refund_gas_callback(max_gas_cost);
    Ok(promise.then(callback))
}

pub fn relay_meta_transactions(relayer: &mut Relayer, signed_delegates: Vec<SignedDelegateAction>) -> Result<Vec<Promise>, RelayerError> {
    if signed_delegates.is_empty() || signed_delegates.len() > CHUNK_SIZE {
        return Err(RelayerError::InvalidNonce);
    }

    let total_gas_cost = 29_000_000_000_000_000_000_000_u128 * signed_delegates.len() as u128;
    if relayer.gas_pool < relayer.min_gas_pool + total_gas_cost {
        RelayerEvent::LowGasPool { remaining: relayer.gas_pool }.emit();
        return Err(RelayerError::InsufficientGasPool);
    }

    let mut promises: Vec<Promise> = signed_delegates.into_iter()
        .map(|signed_delegate| {
            let sender_id = &signed_delegate.delegate_action.sender_id;
            match relayer.auth_accounts.get(sender_id) {
                Some(auth_key) if *auth_key == signed_delegate.public_key => (),
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
                        let mpc_contract = match target_chain.parse() {
                            Ok(account) => account,
                            Err(_) => return Promise::new(env::current_account_id()),
                        };
                        let request = SignRequest {
                            payload,
                            path: derivation_path,
                            key_version: 0,
                        };
                        let args = match borsh::to_vec(&request) {
                            Ok(args) => args,
                            Err(_) => return Promise::new(env::current_account_id()),
                        };
                        promise = Promise::new(mpc_contract)
                            .function_call("sign".to_string(), args, NearToken::from_yoctonear(1), MPC_SIGN_GAS);
                    }
                }
            }
            promise
        })
        .collect();

    relayer.gas_pool -= total_gas_cost;
    promises = promises.into_iter()
        .map(|p| {
            let callback = ext_self::ext(env::current_account_id())
                .with_static_gas(Gas::from_tgas(5))
                .refund_gas_callback(total_gas_cost);
            p.then(callback)
        })
        .collect();
    Ok(promises)
}

pub fn relay_chunked_meta_transactions(relayer: &mut Relayer, signed_delegates: Vec<SignedDelegateAction>) -> Result<Vec<Promise>, RelayerError> {
    if signed_delegates.is_empty() {
        return Err(RelayerError::InvalidNonce);
    }

    let total_gas_cost = 29_000_000_000_000_000_000_000_u128 * signed_delegates.len() as u128;
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
                    Some(auth_key) if *auth_key == signed_delegate.public_key => (),
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
                            let mpc_contract = match target_chain.parse() {
                                Ok(account) => account,
                                Err(_) => return Promise::new(env::current_account_id()),
                            };
                            let request = SignRequest {
                                payload: payload.clone(),
                                path: derivation_path.clone(),
                                key_version: 0,
                            };
                            let args = match borsh::to_vec(&request) {
                                Ok(args) => args,
                                Err(_) => return Promise::new(env::current_account_id()),
                            };
                            promise = Promise::new(mpc_contract)
                                .function_call("sign".to_string(), args, NearToken::from_yoctonear(1), MPC_SIGN_GAS);
                        }
                    }
                }
                promise
            })
            .collect();
        all_promises.extend(chunk_promises);
    }

    relayer.gas_pool -= total_gas_cost;
    all_promises = all_promises.into_iter()
        .map(|p| {
            let callback = ext_self::ext(env::current_account_id())
                .with_static_gas(Gas::from_tgas(5))
                .refund_gas_callback(total_gas_cost);
            p.then(callback)
        })
        .collect();
    Ok(all_promises)
}