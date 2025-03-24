use near_sdk::env;
use near_sdk::json_types::U128;
use crate::state::Relayer;
use crate::events::RelayerEvent;

impl Relayer {
    pub fn deposit_gas_pool(&mut self) {
        let amount = env::attached_deposit();
        assert!(amount.as_yoctonear() > 0, "Deposit must be positive");
        self.gas_pool = self.gas_pool.checked_add(amount).unwrap();
        RelayerEvent::GasPoolDeposited { amount, depositor: env::predecessor_account_id() }.emit();
    }

    pub fn on_receive_near(&mut self) {
        let amount = env::attached_deposit();
        assert!(amount.as_yoctonear() > 0, "Received amount must be positive");
        self.gas_pool = self.gas_pool.checked_add(amount).unwrap();
        RelayerEvent::GasPoolDeposited { amount, depositor: env::predecessor_account_id() }.emit();
    }

    pub fn get_gas_pool(&self) -> U128 {
        U128(self.gas_pool.as_yoctonear())
    }
}