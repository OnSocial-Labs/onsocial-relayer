use near_sdk::{Promise, PublicKey, AccountId, NearToken, Gas};
use serde_json::json;
use crate::state::Relayer;
use crate::errors::RelayerError;

pub fn sponsor_account(relayer: &mut Relayer, account_name: String, public_key: PublicKey) -> Result<Promise, RelayerError> {
    if relayer.gas_pool < relayer.min_gas_pool + relayer.sponsor_amount {
        return Err(RelayerError::InsufficientGasPool);
    }
    let new_account_id: AccountId = format!("{}.near", account_name).parse()
        .map_err(|_| RelayerError::InvalidAccountId)?;
    relayer.gas_pool -= relayer.sponsor_amount;
    let promise = Promise::new("near".parse().unwrap())
        .function_call(
            "create_account".to_string(),
            serde_json::to_vec(&json!({"new_account_id": new_account_id, "new_public_key": public_key})).unwrap(),
            NearToken::from_yoctonear(relayer.sponsor_amount),
            Gas::from_tgas(50),
        );
    Ok(promise)
}