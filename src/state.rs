use near_sdk::store::LookupMap;
use near_sdk::{near, AccountId, PublicKey, BorshStorageKey, Gas};

#[near(serializers=[borsh])]
#[derive(BorshStorageKey)]
pub enum StorageKey {
    AuthAccounts,
    ChainMpcMapping,
}

#[near]
pub struct Relayer {
    pub gas_pool: u128,
    pub min_gas_pool: u128,
    pub max_gas_pool: u128,
    pub sponsor_amount: u128,
    pub offload_recipient: AccountId,
    pub admins: Vec<AccountId>,
    pub auth_accounts: LookupMap<AccountId, PublicKey>,
    pub chain_mpc_mapping: LookupMap<String, AccountId>,
    pub chunk_size: usize,
    pub max_gas: Gas,
    pub mpc_sign_gas: Gas,
    pub callback_gas: Gas,
    pub paused: bool, // New field for pausing
}

impl Relayer {
    pub fn new(admins: Vec<AccountId>, initial_auth_account: AccountId, initial_auth_key: PublicKey, offload_recipient: AccountId) -> Self {
        let mut auth_accounts = LookupMap::new(StorageKey::AuthAccounts);
        auth_accounts.insert(initial_auth_account, initial_auth_key);
        let mut chain_mpc_mapping = LookupMap::new(StorageKey::ChainMpcMapping);
        chain_mpc_mapping.insert("near".to_string(), "mpc.near".parse().unwrap());
        Self {
            gas_pool: 0,
            min_gas_pool: 1_000_000_000_000_000_000_000_000, // 1 NEAR
            max_gas_pool: 500_000_000_000_000_000_000_000_000, // 500 NEAR
            sponsor_amount: 100_000_000_000_000_000_000_000, // 0.1 NEAR
            offload_recipient,
            admins,
            auth_accounts,
            chain_mpc_mapping,
            chunk_size: 5,
            max_gas: Gas::from_tgas(250),
            mpc_sign_gas: Gas::from_tgas(100),
            callback_gas: Gas::from_tgas(10),
            paused: false, // Initially unpaused
        }
    }

    pub fn is_admin(&self, caller: &AccountId) -> bool {
        self.admins.contains(caller)
    }
}