use near_sdk::{env, NearToken, Promise};
use crate::state::Relayer;
use crate::errors::RelayerError;

pub fn deposit_gas_pool(relayer: &mut Relayer) -> Result<(), RelayerError> {
    if relayer.paused {
        return Err(RelayerError::ContractPaused);
    }
    let deposit = env::attached_deposit().as_yoctonear();
    relayer.gas_pool += deposit;

    if relayer.gas_pool > relayer.max_gas_pool {
        let excess = relayer.gas_pool - relayer.max_gas_pool;
        relayer.gas_pool = relayer.max_gas_pool;
        Promise::new(relayer.offload_recipient.clone())
            .transfer(NearToken::from_yoctonear(excess));
    }
    Ok(())
}