use near_sdk::{Promise, Allowance, NearToken, env, ext_contract, AccountId};
use near_sdk::borsh::{self, BorshSerialize, BorshDeserialize};
use core::num::NonZeroU128;
use crate::state::Relayer;
use crate::types::{SignedDelegateAction, Action, SignatureScheme};
use crate::errors::RelayerError;
use crate::events::RelayerEvent;
use near_crypto::{KeyType};
use ed25519_dalek::{Verifier, Signature as Ed25519Signature, VerifyingKey};

#[derive(BorshSerialize, BorshDeserialize)]
struct SignRequest {
    payload: Vec<u8>,
    path: String,
    key_version: u32,
    request_id: u64,
}

#[allow(dead_code)]
#[ext_contract(ext_self)]
trait SelfCallback {
    fn refund_gas_callback(&mut self, initial_cost: u128);
    fn handle_mpc_signature(&mut self, chain: String, request_id: u64, result: Vec<u8>);
    fn handle_bridge_result(&mut self, sender_id: AccountId, action_type: String, result: Vec<u8>);
}

fn verify_signature(signed_delegate: &SignedDelegateAction) -> Result<(), RelayerError> {
    let payload = borsh::to_vec(&signed_delegate.delegate_action).map_err(|_| RelayerError::InvalidNonce)?;
    match signed_delegate.scheme {
        SignatureScheme::Ed25519 => {
            let signature_bytes: [u8; 64] = signed_delegate.signature.clone().try_into().map_err(|_| RelayerError::Unauthorized)?;
            let signature = Ed25519Signature::from_bytes(&signature_bytes);
            let public_key_bytes = signed_delegate.public_key.as_bytes();
            if public_key_bytes.len() != 33 || public_key_bytes[0] != KeyType::ED25519 as u8 {
                return Err(RelayerError::Unauthorized);
            }
            let ed25519_key = VerifyingKey::from_bytes(&public_key_bytes[1..33].try_into().unwrap())
                .map_err(|_| RelayerError::Unauthorized)?;
            ed25519_key.verify(&payload, &signature).map_err(|_| RelayerError::Unauthorized)?;
        }
    }
    Ok(())
}

pub fn relay_meta_transaction(relayer: &mut Relayer, signed_delegate: SignedDelegateAction) -> Result<Promise, RelayerError> {
    if signed_delegate.delegate_action.actions.len() > 1 {
        return Err(RelayerError::InvalidNonce);
    }

    let sender_id = &signed_delegate.delegate_action.sender_id;
    let auth_key = relayer.auth_accounts.get(sender_id).ok_or(RelayerError::Unauthorized)?;
    if *auth_key != signed_delegate.public_key {
        return Err(RelayerError::Unauthorized);
    }

    verify_signature(&signed_delegate)?;

    let max_gas_cost = 29_000_000_000_000_000_000_000_u128;
    let mut total_gas_cost = max_gas_cost;
    if signed_delegate.fee_action.is_some() {
        total_gas_cost += max_gas_cost;
    }

    if relayer.gas_pool < relayer.min_gas_pool + total_gas_cost {
        RelayerEvent::LowGasPool { remaining: relayer.gas_pool }.emit();
        return Err(RelayerError::InsufficientGasPool);
    }

    let delegate = signed_delegate.delegate_action;
    let mut promise = Promise::new(delegate.sender_id.clone());
    let request_id = env::block_timestamp();

    if let Some(fee_action) = signed_delegate.fee_action {
        promise = promise.then(execute_action(relayer, &fee_action, &delegate.sender_id, "FeePayment", None)?);
        promise = promise.then(
            ext_self::ext(env::current_account_id())
                .with_static_gas(relayer.callback_gas)
                .handle_bridge_result(delegate.sender_id.clone(), "FeePayment".to_string(), Vec::new())
        );
    }

    let action = delegate.actions.first().unwrap();
    let request_id_opt = if matches!(action, Action::ChainSignatureRequest { .. }) {
        Some(request_id)
    } else {
        None
    };
    promise = promise.then(execute_action(relayer, action, &delegate.sender_id, action.type_name(), request_id_opt)?);

    promise = match action {
        Action::ChainSignatureRequest { target_chain, .. } => {
            let target_chain = target_chain.clone();
            promise.then(
                ext_self::ext(env::current_account_id())
                    .with_static_gas(relayer.callback_gas)
                    .handle_mpc_signature(target_chain, request_id, Vec::new())
            )
        }
        _ => promise.then(
            ext_self::ext(env::current_account_id())
                .with_static_gas(relayer.callback_gas)
                .handle_bridge_result(delegate.sender_id.clone(), action.type_name().to_string(), Vec::new())
        ),
    };

    relayer.gas_pool -= total_gas_cost;
    Ok(promise.then(
        ext_self::ext(env::current_account_id())
            .with_static_gas(relayer.callback_gas)
            .refund_gas_callback(total_gas_cost)
    ))
}

pub fn relay_meta_transactions(relayer: &mut Relayer, signed_delegates: Vec<SignedDelegateAction>) -> Result<Vec<Promise>, RelayerError> {
    if signed_delegates.is_empty() || signed_delegates.len() > relayer.chunk_size {
        return Err(RelayerError::InvalidNonce);
    }

    let max_gas_cost = 29_000_000_000_000_000_000_000_u128;
    let mut total_gas_cost = 0;
    for delegate in &signed_delegates {
        total_gas_cost += max_gas_cost * delegate.delegate_action.actions.len() as u128;
        if delegate.fee_action.is_some() {
            total_gas_cost += max_gas_cost;
        }
    }

    if relayer.gas_pool < relayer.min_gas_pool + total_gas_cost {
        RelayerEvent::LowGasPool { remaining: relayer.gas_pool }.emit();
        return Err(RelayerError::InsufficientGasPool);
    }

    let mut promises: Vec<Promise> = Vec::new();
    let mut request_id = env::block_timestamp();

    for signed_delegate in signed_delegates {
        let sender_id = &signed_delegate.delegate_action.sender_id;
        let auth_key = relayer.auth_accounts.get(sender_id).ok_or(RelayerError::Unauthorized)?;
        if *auth_key != signed_delegate.public_key {
            return Err(RelayerError::Unauthorized);
        }

        verify_signature(&signed_delegate)?;

        let mut promise = Promise::new(signed_delegate.delegate_action.sender_id.clone());
        if let Some(fee_action) = signed_delegate.fee_action {
            promise = promise.then(execute_action(relayer, &fee_action, sender_id, "FeePayment", None)?);
            promise = promise.then(
                ext_self::ext(env::current_account_id())
                    .with_static_gas(relayer.callback_gas)
                    .handle_bridge_result(sender_id.clone(), "FeePayment".to_string(), Vec::new())
            );
        }

        for action in signed_delegate.delegate_action.actions {
            let current_request_id = if matches!(action, Action::ChainSignatureRequest { .. }) {
                let id = request_id;
                request_id += 1;
                Some(id)
            } else {
                None
            };
            promise = promise.then(execute_action(relayer, &action, sender_id, action.type_name(), current_request_id)?);
            promise = match action {
                Action::ChainSignatureRequest { target_chain, .. } => {
                    let target_chain = target_chain.clone();
                    promise.then(
                        ext_self::ext(env::current_account_id())
                            .with_static_gas(relayer.callback_gas)
                            .handle_mpc_signature(target_chain, current_request_id.unwrap(), Vec::new())
                    )
                }
                _ => promise.then(
                    ext_self::ext(env::current_account_id())
                        .with_static_gas(relayer.callback_gas)
                        .handle_bridge_result(sender_id.clone(), action.type_name().to_string(), Vec::new())
                ),
            };
        }
        promises.push(promise);
    }

    relayer.gas_pool -= total_gas_cost;
    Ok(promises.into_iter().map(|p| {
        p.then(
            ext_self::ext(env::current_account_id())
                .with_static_gas(relayer.callback_gas)
                .refund_gas_callback(total_gas_cost)
        )
    }).collect())
}

pub fn relay_chunked_meta_transactions(relayer: &mut Relayer, signed_delegates: Vec<SignedDelegateAction>) -> Result<Vec<Promise>, RelayerError> {
    if signed_delegates.is_empty() {
        return Err(RelayerError::InvalidNonce);
    }

    let max_gas_cost = 29_000_000_000_000_000_000_000_u128;
    let mut total_gas_cost = 0;
    for delegate in &signed_delegates {
        total_gas_cost += max_gas_cost * delegate.delegate_action.actions.len() as u128;
        if delegate.fee_action.is_some() {
            total_gas_cost += max_gas_cost;
        }
    }

    if relayer.gas_pool < relayer.min_gas_pool + total_gas_cost {
        RelayerEvent::LowGasPool { remaining: relayer.gas_pool }.emit();
        return Err(RelayerError::InsufficientGasPool);
    }

    let mut all_promises = Vec::new();
    let mut request_id = env::block_timestamp();

    for chunk in signed_delegates.chunks(relayer.chunk_size) {
        let chunk_promises: Vec<Promise> = chunk.iter()
            .map(|signed_delegate| {
                let sender_id = &signed_delegate.delegate_action.sender_id;
                match relayer.auth_accounts.get(sender_id) {
                    Some(auth_key) if *auth_key == signed_delegate.public_key => (),
                    _ => return Ok(Promise::new(env::current_account_id())),
                }

                if verify_signature(signed_delegate).is_err() {
                    return Ok(Promise::new(env::current_account_id()));
                }

                let mut promise = Promise::new(signed_delegate.delegate_action.sender_id.clone());
                if let Some(fee_action) = &signed_delegate.fee_action {
                    promise = promise.then(execute_action(relayer, fee_action, sender_id, "FeePayment", None)?);
                    promise = promise.then(
                        ext_self::ext(env::current_account_id())
                            .with_static_gas(relayer.callback_gas)
                            .handle_bridge_result(sender_id.clone(), "FeePayment".to_string(), Vec::new())
                    );
                }

                for action in &signed_delegate.delegate_action.actions {
                    let current_request_id = if matches!(action, Action::ChainSignatureRequest { .. }) {
                        let id = request_id;
                        request_id += 1;
                        Some(id)
                    } else {
                        None
                    };
                    promise = promise.then(execute_action(relayer, action, sender_id, action.type_name(), current_request_id)?);
                    promise = match action {
                        Action::ChainSignatureRequest { target_chain, .. } => {
                            let target_chain = target_chain.clone();
                            promise.then(
                                ext_self::ext(env::current_account_id())
                                    .with_static_gas(relayer.callback_gas)
                                    .handle_mpc_signature(target_chain, current_request_id.unwrap(), Vec::new())
                            )
                        }
                        _ => promise.then(
                            ext_self::ext(env::current_account_id())
                                .with_static_gas(relayer.callback_gas)
                                .handle_bridge_result(sender_id.clone(), action.type_name().to_string(), Vec::new())
                        ),
                    };
                }
                Ok(promise)
            })
            .collect::<Result<Vec<_>, RelayerError>>()?;
        all_promises.extend(chunk_promises);
    }

    relayer.gas_pool -= total_gas_cost;
    Ok(all_promises.into_iter().map(|p| {
        p.then(
            ext_self::ext(env::current_account_id())
                .with_static_gas(relayer.callback_gas)
                .refund_gas_callback(total_gas_cost)
        )
    }).collect())
}

fn execute_action(
    relayer: &Relayer,
    action: &Action,
    sender_id: &AccountId,
    _action_type: &str,
    request_id: Option<u64>,
) -> Result<Promise, RelayerError> {
    let mut promise = Promise::new(sender_id.clone());
    
    match action {
        Action::FunctionCall { method_name, args, gas, deposit } => {
            let capped_gas = if *gas > relayer.max_gas { relayer.max_gas } else { *gas };
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
            let mpc_contract = relayer.chain_mpc_mapping.get(target_chain)
                .ok_or(RelayerError::InvalidAccountId)?.clone();
            let request = SignRequest {
                payload: payload.clone(),
                path: derivation_path.clone(),
                key_version: 0,
                request_id: request_id.ok_or(RelayerError::InvalidNonce)?,
            };
            let args = borsh::to_vec(&request).map_err(|_| RelayerError::InvalidAccountId)?;
            promise = Promise::new(mpc_contract)
                .function_call("sign".to_string(), args, NearToken::from_yoctonear(1), relayer.mpc_sign_gas);
        }
    }

    Ok(promise)
}

impl Action {
    fn type_name(&self) -> &str {
        match self {
            Action::ChainSignatureRequest { .. } => "ChainSignatureRequest",
            Action::FunctionCall { .. } => "FunctionCall",
            Action::Transfer { .. } => "Transfer",
            Action::AddKey { .. } => "AddKey",
        }
    }
}