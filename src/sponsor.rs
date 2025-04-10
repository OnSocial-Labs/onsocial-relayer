use near_sdk::{env, Promise, PublicKey, AccountId, NearToken, Gas, borsh::{self, BorshSerialize}};
use crate::state::Relayer;
use crate::errors::RelayerError;
use crate::relay::{ext_self, verify_signature};
use crate::types::{Action, SignedDelegateAction};

#[derive(BorshSerialize)]
struct CreateAccountArgs {
    new_account_id: AccountId,
    new_public_key: PublicKey,
}

pub fn sponsor_account(relayer: &mut Relayer, new_account_id: AccountId, public_key: PublicKey) -> Result<Promise, RelayerError> {
    let caller = env::predecessor_account_id();
    if relayer.paused || !relayer.auth_accounts.contains_key(&caller) {
        return Err(if relayer.paused { RelayerError::ContractPaused } else { RelayerError::Unauthorized });
    }
    sponsor_account_inner(relayer, new_account_id, public_key)
}

pub fn sponsor_account_signed(relayer: &mut Relayer, signed_delegate: SignedDelegateAction) -> Result<Promise, RelayerError> {
    if relayer.paused {
        return Err(RelayerError::ContractPaused);
    }
    let delegate = &signed_delegate.delegate_action;
    let sender_id = &delegate.sender_id;

    let auth_key = relayer.auth_accounts.get(sender_id).ok_or(RelayerError::Unauthorized)?;
    if *auth_key != signed_delegate.public_key {
        return Err(RelayerError::Unauthorized);
    }
    verify_signature(&signed_delegate)?;

    if delegate.actions.len() != 1 {
        return Err(RelayerError::InvalidNonce);
    }
    match &delegate.actions[0] {
        Action::AddKey { public_key, .. } => {
            let new_account_id = delegate.receiver_id.clone();
            // For signed actions, we assume the receiver_id is the new account to create
            sponsor_account_inner(relayer, new_account_id, public_key.clone())
        }
        _ => Err(RelayerError::InvalidNonce),
    }
}

fn sponsor_account_inner(relayer: &mut Relayer, new_account_id: AccountId, public_key: PublicKey) -> Result<Promise, RelayerError> {
    if relayer.gas_pool < relayer.min_gas_pool + relayer.sponsor_amount {
        return Err(RelayerError::InsufficientGasPool);
    }

    relayer.gas_pool -= relayer.sponsor_amount;

    let args = CreateAccountArgs {
        new_account_id: new_account_id.clone(),
        new_public_key: public_key,
    };
    let args_serialized = borsh::to_vec(&args).map_err(|_| RelayerError::InvalidAccountId)?;

    let max_call_gas = if relayer.max_gas.as_tgas() > 270 { Gas::from_tgas(270) } else { relayer.max_gas };
    let max_callback_gas = if relayer.callback_gas.as_tgas() > 10 { Gas::from_tgas(10) } else { relayer.callback_gas };
    let total_gas = max_call_gas.as_tgas() + max_callback_gas.as_tgas();

    let promise = if total_gas > 280 {
        let adjusted_call_gas = Gas::from_tgas(280 - max_callback_gas.as_tgas());
        Promise::new(relayer.registrar.clone())
            .function_call(
                "create_account".to_string(),
                args_serialized,
                NearToken::from_yoctonear(relayer.sponsor_amount),
                adjusted_call_gas,
            )
    } else {
        Promise::new(relayer.registrar.clone())
            .function_call(
                "create_account".to_string(),
                args_serialized,
                NearToken::from_yoctonear(relayer.sponsor_amount),
                max_call_gas,
            )
    };

    Ok(promise.then(
        ext_self::ext(env::current_account_id())
            .with_static_gas(max_callback_gas)
            .refund_gas_callback(relayer.sponsor_amount)
    ))
}