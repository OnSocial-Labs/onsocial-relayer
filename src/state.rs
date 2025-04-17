use near_sdk::{AccountId, env};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::store::{LazyOption, LookupMap, IterableSet};
use near_sdk_macros::NearSchema;

#[derive(BorshDeserialize, BorshSerialize, NearSchema)]
#[abi(borsh)]
pub struct RelayerV1 {
    pub admins: IterableSet<AccountId>,
    pub offload_recipient: AccountId,
    pub auth_contract: AccountId,
    pub ft_wrapper_contract: AccountId,
    pub version: String,
}

#[derive(BorshDeserialize, BorshSerialize, NearSchema)]
#[abi(borsh)]
pub struct Relayer {
    pub admins: IterableSet<AccountId>,
    pub offload_recipient: AccountId,
    pub auth_contract: AccountId,
    pub ft_wrapper_contract: AccountId,
    pub omni_locker_contract: LazyOption<AccountId>,
    pub chain_mpc_mapping: LookupMap<String, AccountId>,
    pub sponsor_amount: u128,
    pub sponsor_gas: u64,
    pub cross_contract_gas: u64,
    pub chunk_size: usize,
    pub paused: bool,
    pub version: String,
    pub migration_version: u64,
    pub min_balance: u128,
    pub max_balance: u128,
    pub base_fee: u128,
    pub transfer_nonces: LookupMap<String, u64>,
}

impl From<RelayerV1> for Relayer {
    fn from(v1: RelayerV1) -> Self {
        Self {
            admins: v1.admins,
            offload_recipient: v1.offload_recipient,
            auth_contract: v1.auth_contract,
            ft_wrapper_contract: v1.ft_wrapper_contract,
            omni_locker_contract: LazyOption::new(b"omni_locker".to_vec(), Some(env::current_account_id())),
            chain_mpc_mapping: LookupMap::new(b"chain_mpc".to_vec()),
            sponsor_amount: 10_000_000_000_000_000_000_000,
            sponsor_gas: 100_000_000_000_000,
            cross_contract_gas: 100_000_000_000_000,
            chunk_size: 10,
            paused: false,
            version: v1.version,
            migration_version: 0,
            min_balance: 10_000_000_000_000_000_000_000_000,
            max_balance: 1_000_000_000_000_000_000_000_000_000,
            base_fee: 100_000_000_000_000_000_000, // 0.0001 NEAR
            transfer_nonces: LookupMap::new(b"nonces".to_vec()),
        }
    }
}

impl Relayer {
    pub fn new(
        admins: Vec<AccountId>,
        offload_recipient: AccountId,
        auth_contract: AccountId,
        ft_wrapper_contract: AccountId,
    ) -> Self {
        let mut admin_set = IterableSet::new(b"admins".to_vec());
        for admin in admins {
            admin_set.insert(admin);
        }
        Self {
            admins: admin_set,
            offload_recipient,
            auth_contract,
            ft_wrapper_contract,
            omni_locker_contract: LazyOption::new(b"omni_locker".to_vec(), Some(env::current_account_id())),
            chain_mpc_mapping: LookupMap::new(b"chain_mpc".to_vec()),
            sponsor_amount: 10_000_000_000_000_000_000_000,
            sponsor_gas: 100_000_000_000_000,
            cross_contract_gas: 100_000_000_000_000,
            chunk_size: 10,
            paused: false,
            version: "1.0".to_string(),
            migration_version: 0,
            min_balance: 10_000_000_000_000_000_000_000_000,
            max_balance: 1_000_000_000_000_000_000_000_000_000,
            base_fee: 100_000_000_000_000_000_000, // 0.0001 NEAR
            transfer_nonces: LookupMap::new(b"nonces".to_vec()),
        }
    }

    pub fn is_admin(&self, account_id: &AccountId) -> bool {
        self.admins.contains(account_id)
    }

    pub fn get_next_nonce(&mut self, chain: &str) -> u64 {
        let nonce = self.transfer_nonces.get(chain).copied().unwrap_or(0);
        self.transfer_nonces.insert(chain.to_string(), nonce + 1);
        nonce
    }
}