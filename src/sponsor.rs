use near_sdk::{env, Promise, PublicKey, AccountId, NearToken, Gas};
use crate::state::Relayer;
use crate::errors::RelayerError;
use crate::relay::{ext_self, verify_signature};
use crate::types::{Action, SignedDelegateAction};

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
            sponsor_account_inner(relayer, new_account_id, public_key.clone())
        }
        _ => Err(RelayerError::InvalidNonce),
    }
}

fn sponsor_account_inner(relayer: &mut Relayer, new_account_id: AccountId, public_key: PublicKey) -> Result<Promise, RelayerError> {
    if relayer.gas_pool < relayer.min_gas_pool + relayer.sponsor_amount {
        return Err(RelayerError::InsufficientGasPool);
    }

    // Use relayer.max_gas and relayer.callback_gas directly, ensuring total <= 300 TGas
    let prepaid_gas = env::prepaid_gas().as_gas();
    let max_call_gas = relayer.max_gas;
    let max_callback_gas = relayer.callback_gas;
    let total_required_gas = max_call_gas.as_gas() + max_callback_gas.as_gas();
    let max_allowed_gas = Gas::from_tgas(300).as_gas();

    if total_required_gas > max_allowed_gas {
        return Err(RelayerError::InsufficientGasPool); // Total exceeds 300 TGas
    }
    if prepaid_gas < total_required_gas {
        return Err(RelayerError::InsufficientGasPool); // Prepaid gas insufficient
    }

    relayer.gas_pool -= relayer.sponsor_amount;

    let promise = Promise::new(new_account_id.clone())
        .create_account()
        .add_full_access_key(public_key)
        .transfer(NearToken::from_yoctonear(relayer.sponsor_amount));

    Ok(promise.then(
        ext_self::ext(env::current_account_id())
            .with_static_gas(max_callback_gas)
            .refund_gas_callback(relayer.sponsor_amount)
    ))
}