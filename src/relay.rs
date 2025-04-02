use near_sdk::{Promise, Allowance, AccountId, NearToken, Gas};
use serde_json::json;
use crate::state::Relayer;
use crate::types::{SignedDelegateAction, Action};
use crate::errors::RelayerError;
use core::num::NonZeroU128;

const MAX_GAS: Gas = Gas::from_tgas(300); // 300 TGas

pub fn relay_meta_transaction(relayer: &mut Relayer, signed_delegate: SignedDelegateAction) -> Result<Promise, RelayerError> {
    // Check gas pool sufficiency
    if relayer.gas_pool < relayer.min_gas_pool {
        return Err(RelayerError::InsufficientGasPool);
    }
    
    let delegate = signed_delegate.delegate_action;
    // Verify sender is authorized
    if !relayer.auth_accounts.contains_key(&delegate.sender_id) {
        return Err(RelayerError::Unauthorized);
    }

    // Pre-check actions: fail fast if empty
    if delegate.actions.is_empty() {
        return Err(RelayerError::InsufficientGasPool); // Reuse error for simplicity
    }

    // Build promise based on actions
    let mut promise = Promise::new(delegate.receiver_id.clone());
    let mut gas_cost = 0; // In yoctoNEAR
    for action in delegate.actions {
        match action {
            Action::FunctionCall { method_name, args, gas, deposit } => {
                let capped_gas = if gas > MAX_GAS { MAX_GAS } else { gas };
                promise = promise.function_call(method_name, args, deposit, capped_gas);
                gas_cost = 5_000_000_000_000_000_000_000; // ~50 TGas ≈ 0.005 NEAR
            }
            Action::Transfer { deposit } => {
                promise = promise.transfer(deposit);
                gas_cost = 1_000_000_000_000_000_000_000; // ~10 TGas ≈ 0.001 NEAR
            }
            Action::AddKey { public_key, allowance, receiver_id, method_names } => {
                promise = promise.add_access_key_allowance(
                    public_key,
                    allowance.map_or(Allowance::Unlimited, |t| Allowance::Limited(NonZeroU128::new(t.as_yoctonear()).unwrap())),
                    receiver_id,
                    method_names.join(",")
                );
                gas_cost = 10_000_000_000_000_000_000_000; // ~100 TGas ≈ 0.01 NEAR
            }
            Action::ChainSignatureRequest { target_chain, derivation_path, payload } => {
                let mpc_contract: AccountId = target_chain.split('|').last().unwrap_or(&target_chain).parse()
                    .map_err(|_| RelayerError::InvalidAccountId)?;
                let args = serde_json::to_vec(&json!({"request": {"payload": payload, "path": derivation_path, "key_version": 0}}))
                    .map_err(|_| RelayerError::InvalidAccountId)?;
                promise = Promise::new(mpc_contract)
                    .function_call("sign".to_string(), args, NearToken::from_yoctonear(1), MAX_GAS);
                gas_cost = 20_000_000_000_000_000_000_000; // ~200 TGas ≈ 0.02 NEAR
            }
        }
    }

    // Deduct based on action cost
    relayer.gas_pool = relayer.gas_pool.saturating_sub(gas_cost);
    Ok(promise)
}

pub fn relay_meta_transactions(relayer: &mut Relayer, signed_delegates: Vec<SignedDelegateAction>) -> Result<Vec<Promise>, RelayerError> {
    // Pre-validate batch size
    let batch_size = signed_delegates.len() as u128;
    if relayer.gas_pool < relayer.min_gas_pool * batch_size || batch_size == 0 {
        return Err(RelayerError::InsufficientGasPool);
    }

    let promises: Vec<Promise> = signed_delegates.into_iter()
        .map(|signed_delegate| relay_meta_transaction(relayer, signed_delegate))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(promises)
}