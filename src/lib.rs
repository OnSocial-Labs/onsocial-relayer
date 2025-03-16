// Core NEAR SDK imports providing essential blockchain functionality
use near_sdk::{env, near_bindgen, AccountId, NearToken, Promise, PanicOnDefault, PromiseResult};
// Collection type for persistent storage of account creation data
use near_sdk::collections::LookupMap;
// JSON-serializable wrapper for large numbers (e.g., yoctoNEAR amounts)
use near_sdk::json_types::U128;
// Borsh serialization/deserialization for contract state persistence
use borsh::{self, BorshSerialize, BorshDeserialize};
// Gas units for managing transaction costs
use near_gas::NearGas;

// Constants defining token amounts and gas limits for account creation
const MINIMUM_SUBACCOUNT_BALANCE: NearToken = NearToken::from_yoctonear(1_000_000_000_000_000_000_000); // 0.001 NEAR, default funding for Testnet subaccounts
const SPONSORED_SUBACCOUNT_BALANCE: NearToken = NearToken::from_yoctonear(50_000_000_000_000_000_000_000); // 0.05 NEAR, sponsored amount for Mainnet .near subaccounts
const MINIMUM_TOPLEVEL_BALANCE: NearToken = NearToken::from_yoctonear(100_000_000_000_000_000_000_000); // 0.1 NEAR, minimum funding for top-level accounts
const EXTRA_STORAGE_COST: NearToken = NearToken::from_yoctonear(300_000_000_000_000_000_000); // ~0.0003 NEAR, additional buffer for storage costs
const CALLBACK_GAS: NearGas = NearGas::from_tgas(100); // 100 TGas for callbacks, (typical usage <50 TGas)
const PENDING_BLOCK_HEIGHT: u64 = u64::MAX; // Special value marking accounts as pending creation

/// OnSocialRelayer: A NEAR smart contract for creating subaccounts and top-level accounts.
/// - Sponsors 0.05 NEAR for .onsocial.near subaccounts on Mainnet.
/// - Uses configurable balances (default 0.001 NEAR for Testnet, 0.1 NEAR for top-level).
/// - Tracks creation status with callbacks and logs events using NEP-297.
#[near_bindgen] // Marks this as a NEAR contract, enabling blockchain integration
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)] // Serialization and default panic if uninitialized
pub struct OnSocialRelayer {
    created_accounts: LookupMap<AccountId, u64>, // Stores account IDs mapped to creation block height (or PENDING_BLOCK_HEIGHT)
    subaccount_balance: NearToken, // Configurable funding amount for subaccounts, defaults to 0.001 NEAR
    toplevel_balance: NearToken, // Configurable funding amount for top-level accounts, defaults to 0.1 NEAR
    owner: AccountId, // Account that deployed the contract, with privileged access
}

#[near_bindgen]
impl OnSocialRelayer {
    /// Initializes the contract with default balances and sets the deployer as owner.
    #[init] // Marks this as the initialization function, called during deployment
    pub fn new() -> Self {
        Self {
            created_accounts: LookupMap::new(b"c"), // Initializes storage with prefix "c" for account tracking
            subaccount_balance: MINIMUM_SUBACCOUNT_BALANCE, // Sets default subaccount funding
            toplevel_balance: MINIMUM_TOPLEVEL_BALANCE, // Sets default top-level funding
            owner: env::predecessor_account_id(), // Sets owner to the account calling `new`
        }
    }

    /// Creates a subaccount (e.g., alice.onsocial.near) with a public key.
    /// - Sponsors 0.05 NEAR for Mainnet (.near), otherwise uses subaccount_balance.
    /// - Returns a Promise that triggers a callback to confirm success or handle failure.
    pub fn create_account(&mut self, new_account_id: AccountId, public_key: String) -> Promise {
        self.assert_app_wallet(); // Ensures only the relayer itself can create accounts
        Self::validate_subaccount_id(&new_account_id); // Verifies the account ID is a valid subaccount
        Self::validate_public_key(&public_key); // Checks the public key format
        self.check_duplicate(&new_account_id); // Prevents recreating an already existing account

        let parsed_key = public_key.parse().expect("Invalid public key format"); // Parses the key, panics if invalid (already validated)
        let transfer_amount = if env::current_account_id().as_str().ends_with(".near") {
            SPONSORED_SUBACCOUNT_BALANCE // Mainnet: sponsors 0.05 NEAR
        } else {
            self.subaccount_balance // Testnet: uses configurable amount (default 0.001 NEAR)
        };
        let required_balance = self.calculate_required_balance(transfer_amount); // Adds storage costs to transfer amount
        self.assert_sufficient_funds(required_balance); // Ensures the relayer has enough NEAR

        self.created_accounts.insert(&new_account_id, &PENDING_BLOCK_HEIGHT); // Marks account as pending

        Promise::new(new_account_id.clone()) // Creates a new NEAR Promise to execute the account creation
            .create_account() // Initializes the new account
            .add_full_access_key(parsed_key) // Grants the provided key full access
            .transfer(transfer_amount) // Transfers the calculated NEAR amount
            .then( // Chains a callback to handle the result
                Self::ext(env::current_account_id()) // Calls back to this contract
                    .with_static_gas(CALLBACK_GAS) // Allocates 100 TGas for the callback
                    .on_account_created(new_account_id, transfer_amount) // Executes the callback function
            )
    }

    /// Creates a top-level account (e.g., alice.testnet) with a public key.
    /// - Transfers the configurable toplevel_balance (default 0.1 NEAR).
    /// - Returns a Promise with a callback to track the outcome.
    pub fn create_top_level_account(&mut self, new_account_id: AccountId, public_key: String) -> Promise {
        self.assert_app_wallet(); // Restricts to relayer-only calls
        Self::validate_toplevel_id(&new_account_id); // Ensures the ID isn’t a subaccount of the relayer
        Self::validate_public_key(&public_key); // Validates the public key
        self.check_duplicate(&new_account_id); // Checks for duplicates

        let parsed_key = public_key.parse().expect("Invalid public key format");
        let required_balance = self.calculate_required_balance(self.toplevel_balance);
        self.assert_sufficient_funds(required_balance);

        self.created_accounts.insert(&new_account_id, &PENDING_BLOCK_HEIGHT);

        Promise::new(new_account_id.clone())
            .create_account()
            .add_full_access_key(parsed_key)
            .transfer(self.toplevel_balance) // Uses toplevel_balance instead of a sponsored amount
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(CALLBACK_GAS)
                    .on_account_created(new_account_id, self.toplevel_balance)
            )
    }

    /// Callback function (private) to handle account creation results.
    /// - Success: Updates the account’s block height and logs an event.
    /// - Failure: Removes the pending status, refunds NEAR, and logs the failure.
    #[private] // Only callable by the contract itself (via Promise)
    pub fn on_account_created(&mut self, new_account_id: AccountId, transferred_balance: NearToken) {
        match env::promise_result(0) { // Checks the result of the previous Promise
            PromiseResult::Successful(_) => {
                self.created_accounts.insert(&new_account_id, &env::block_height()); // Records the creation block height
                env::log_str(&format!( // Logs a NEP-297 compliant event
                    r#"{{"standard": "nep297", "version": "1.0.0", "event": "AccountCreated", "data": {{"account_id": "{}"}}}}"#,
                    new_account_id
                ));
            }
            PromiseResult::Failed => {
                self.created_accounts.remove(&new_account_id); // Cleans up pending status
                env::log_str(&format!(
                    r#"{{"standard": "nep297", "version": "1.0.0", "event": "AccountCreationFailed", "data": {{"account_id": "{}", "refunded_amount": "{}"}}}}"#,
                    new_account_id, transferred_balance.as_yoctonear()
                ));
                Promise::new(env::current_account_id()).transfer(transferred_balance); // Refunds the NEAR to the relayer
            }
        }
    }

    /// Allows the owner to set a new subaccount balance, with a minimum of 0.001 NEAR.
    pub fn set_subaccount_balance(&mut self, amount: U128) {
        self.assert_owner(); // Restricts to owner only
        let new_balance = NearToken::from_yoctonear(amount.0); // Converts from yoctoNEAR
        assert!(
            new_balance >= MINIMUM_SUBACCOUNT_BALANCE,
            "Sub-account balance must be at least {}",
            MINIMUM_SUBACCOUNT_BALANCE.as_yoctonear()
        );
        self.subaccount_balance = new_balance; // Updates the balance
    }

    /// Allows the owner to set a new top-level balance, with a minimum of 0.1 NEAR.
    pub fn set_toplevel_balance(&mut self, amount: U128) {
        self.assert_owner();
        let new_balance = NearToken::from_yoctonear(amount.0);
        assert!(
            new_balance >= MINIMUM_TOPLEVEL_BALANCE,
            "Top-level balance must be at least {}",
            MINIMUM_TOPLEVEL_BALANCE.as_yoctonear()
        );
        self.toplevel_balance = new_balance;
    }

    /// Returns the current subaccount balance in yoctoNEAR for external querying.
    pub fn get_subaccount_balance(&self) -> U128 {
        U128(self.subaccount_balance.as_yoctonear())
    }

    /// Returns the current top-level balance in yoctoNEAR.
    pub fn get_toplevel_balance(&self) -> U128 {
        U128(self.toplevel_balance.as_yoctonear())
    }

    /// Returns the contract owner’s AccountId.
    pub fn get_owner(&self) -> AccountId {
        self.owner.clone()
    }

    /// Checks if an account has been successfully created (i.e., not pending).
    pub fn has_created_account(&self, account: AccountId) -> bool {
        self.created_accounts.get(&account).map_or(false, |height| height != PENDING_BLOCK_HEIGHT)
    }

    /// Returns the contract’s current NEAR balance in yoctoNEAR.
    pub fn get_contract_balance(&self) -> U128 {
        U128(env::account_balance().as_yoctonear())
    }

    /// Validates that an account ID is a proper subaccount of .onsocial.near or .onsocial.testnet.
    fn validate_subaccount_id(account_id: &AccountId) {
        let parent_account = if env::current_account_id().as_str().ends_with(".testnet") {
            "onsocial.testnet" // Testnet parent
        } else {
            "onsocial.near" // Mainnet parent
        };
        let account_str = account_id.as_str();
        let parent_str = parent_account;
        assert!(
            account_str.ends_with(parent_str) && account_str.len() > parent_str.len() && account_str[..account_str.len() - parent_str.len()].ends_with("."),
            "Sub-account must be a valid subaccount of '{}'",
            parent_str
        );
    }

    /// Ensures a top-level account ID isn’t a subaccount of the relayer and meets length requirements.
    fn validate_toplevel_id(account_id: &AccountId) {
        let current = env::current_account_id();
        let suffix = if current.as_str().ends_with(".testnet") { "testnet" } else { "near" };
        assert!(
            !account_id.as_str().ends_with(&format!(".{}", current.as_str())),
            "Use create_account for {}.{} sub-accounts",
            current.as_str(),
            suffix
        );
        assert!(
            account_id.len() > 2,
            "Top-level account ID must be longer than 2 characters"
        );
    }

    /// Validates that a public key is 52 characters long and starts with "ed25519:".
    fn validate_public_key(public_key: &String) {
        assert!(
            public_key.len() == 52 && public_key.starts_with("ed25519:"),
            "Invalid public key: must be 52 characters and start with 'ed25519:'"
        );
    }

    /// Prevents duplicate account creation by checking if the account already exists (not pending).
    fn check_duplicate(&self, account_id: &AccountId) {
        if let Some(block_height) = self.created_accounts.get(account_id) {
            if block_height != PENDING_BLOCK_HEIGHT {
                env::panic_str(&format!(
                    "Account {} already created at block height {}",
                    account_id, block_height
                ));
            }
        }
    }

    /// Calculates the total NEAR required for account creation, including storage costs.
    fn calculate_required_balance(&self, base_balance: NearToken) -> NearToken {
        let storage_cost = NearToken::from_yoctonear(
            env::storage_usage() as u128 * env::storage_byte_cost().as_yoctonear() // Dynamic storage cost
        );
        base_balance
            .saturating_add(storage_cost) // Adds storage cost, preventing overflow
            .saturating_add(EXTRA_STORAGE_COST) // Adds a buffer for safety
    }

    /// Ensures the contract has sufficient funds to cover the required balance.
    fn assert_sufficient_funds(&self, required_balance: NearToken) {
        let contract_balance = env::account_balance();
        assert!(
            contract_balance >= required_balance,
            "Insufficient funds in relayer: {} available, {} required",
            contract_balance.as_near(),
            required_balance.as_near()
        );
    }

    /// Restricts an action to the contract owner only.
    fn assert_owner(&self) {
        assert_eq!(
            env::predecessor_account_id(),
            self.owner,
            "Only owner can perform this action"
        );
    }

    /// Ensures only the relayer contract itself can initiate account creation.
    fn assert_app_wallet(&self) {
        let current = env::current_account_id();
        assert_eq!(
            env::predecessor_account_id().as_str(),
            current.as_str(),
            "Only {} can create accounts",
            current.as_str()
        );
    }

    /// Migration function to update contract state during upgrades (private, ignores old state).
    #[private]
    #[init(ignore_state)] // Special initialization for migrations
    pub fn migrate() -> Self {
        let old: Self = env::state_read().expect("Failed to read state"); // Reads existing state
        Self {
            created_accounts: old.created_accounts,
            subaccount_balance: old.subaccount_balance,
            toplevel_balance: old.toplevel_balance,
            owner: old.owner,
        }
    }
}

// Unit tests for validation and functionality (fully documented in the original)
#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use near_sdk::test_utils::{VMContextBuilder, accounts};
    use near_sdk::{testing_env, VMContext};

    const VALID_PUBLIC_KEY: &str = "ed25519:DcA2MzgpJbrUATQLLceocVckhhAqrkingax4oJ9kZ847";

    fn setup_context(
        predecessor: AccountId,
        current: AccountId,
        balance: NearToken,
        is_view: bool,
    ) -> VMContext {
        let mut builder = VMContextBuilder::new();
        builder
            .predecessor_account_id(predecessor)
            .current_account_id(current)
            .account_balance(balance)
            .block_height(100)
            .is_view(is_view);
        builder.build()
    }

    #[test]
    fn test_new() {
        let context = setup_context(
            "relayer.onsocial.testnet".parse().unwrap(),
            "relayer.onsocial.testnet".parse().unwrap(),
            NearToken::from_near(5),
            false,
        );
        testing_env!(context);
        let contract = OnSocialRelayer::new();
        assert_eq!(contract.get_owner(), "relayer.onsocial.testnet".parse::<AccountId>().unwrap());
        assert_eq!(contract.get_subaccount_balance().0, MINIMUM_SUBACCOUNT_BALANCE.as_yoctonear());
        assert_eq!(contract.get_toplevel_balance().0, MINIMUM_TOPLEVEL_BALANCE.as_yoctonear());
    }

    #[test]
    fn test_create_subaccount_initial_state() {
        let context = setup_context(
            "relayer.onsocial.testnet".parse().unwrap(),
            "relayer.onsocial.testnet".parse().unwrap(),
            NearToken::from_near(5),
            false,
        );
        testing_env!(context);
        let mut contract = OnSocialRelayer::new();
        let subaccount: AccountId = "alice.onsocial.testnet".parse().unwrap();

        contract.create_account(subaccount.clone(), VALID_PUBLIC_KEY.to_string());
        assert!(!contract.has_created_account(subaccount)); // Still pending
    }

    #[test]
    fn test_create_toplevel_initial_state() {
        let context = setup_context(
            "relayer.onsocial.testnet".parse().unwrap(),
            "relayer.onsocial.testnet".parse().unwrap(),
            NearToken::from_near(5),
            false,
        );
        testing_env!(context);
        let mut contract = OnSocialRelayer::new();
        let toplevel: AccountId = "alice.testnet".parse().unwrap();

        contract.create_top_level_account(toplevel.clone(), VALID_PUBLIC_KEY.to_string());
        assert!(!contract.has_created_account(toplevel)); // Still pending
    }

    #[test]
    #[should_panic(expected = "Only relayer.onsocial.testnet can create accounts")]
    fn test_subaccount_wrong_caller() {
        let context = setup_context(
            accounts(0),
            "relayer.onsocial.testnet".parse().unwrap(),
            NearToken::from_near(5),
            false,
        );
        testing_env!(context);
        let mut contract = OnSocialRelayer::new();
        let subaccount: AccountId = "alice.onsocial.testnet".parse().unwrap();

        contract.create_account(subaccount, VALID_PUBLIC_KEY.to_string());
    }

    #[test]
    #[should_panic(expected = "Only relayer.onsocial.testnet can create accounts")]
    fn test_toplevel_wrong_caller() {
        let context = setup_context(
            accounts(0),
            "relayer.onsocial.testnet".parse().unwrap(),
            NearToken::from_near(5),
            false,
        );
        testing_env!(context);
        let mut contract = OnSocialRelayer::new();
        let toplevel: AccountId = "alice.testnet".parse().unwrap();

        contract.create_top_level_account(toplevel, VALID_PUBLIC_KEY.to_string());
    }

    #[test]
    #[should_panic(expected = "Invalid public key")]
    fn test_invalid_public_key_subaccount() {
        let context = setup_context(
            "relayer.onsocial.testnet".parse().unwrap(),
            "relayer.onsocial.testnet".parse().unwrap(),
            NearToken::from_near(5),
            false,
        );
        testing_env!(context);
        let mut contract = OnSocialRelayer::new();
        let subaccount: AccountId = "alice.onsocial.testnet".parse().unwrap();

        contract.create_account(subaccount, "invalid_key".to_string());
    }

    #[test]
    #[should_panic(expected = "Invalid public key")]
    fn test_invalid_public_key_toplevel() {
        let context = setup_context(
            "relayer.onsocial.testnet".parse().unwrap(),
            "relayer.onsocial.testnet".parse().unwrap(),
            NearToken::from_near(5),
            false,
        );
        testing_env!(context);
        let mut contract = OnSocialRelayer::new();
        let toplevel: AccountId = "alice.testnet".parse().unwrap();

        contract.create_top_level_account(toplevel, "invalid_key".to_string());
    }

    #[test]
    #[should_panic(expected = "Sub-account must be a valid subaccount of 'onsocial.testnet'")]
    fn test_invalid_subaccount_id() {
        let context = setup_context(
            "relayer.onsocial.testnet".parse().unwrap(),
            "relayer.onsocial.testnet".parse().unwrap(),
            NearToken::from_near(5),
            false,
        );
        testing_env!(context);
        let mut contract = OnSocialRelayer::new();
        let subaccount: AccountId = "alice.testnet".parse().unwrap();

        contract.create_account(subaccount, VALID_PUBLIC_KEY.to_string());
    }

    #[test]
    #[should_panic(expected = "Use create_account for relayer.onsocial.testnet.testnet sub-accounts")]
    fn test_invalid_toplevel_id() {
        let context = setup_context(
            "relayer.onsocial.testnet".parse().unwrap(),
            "relayer.onsocial.testnet".parse().unwrap(),
            NearToken::from_near(5),
            false,
        );
        testing_env!(context);
        let mut contract = OnSocialRelayer::new();
        let toplevel: AccountId = "alice.relayer.onsocial.testnet".parse().unwrap();

        contract.create_top_level_account(toplevel, VALID_PUBLIC_KEY.to_string());
    }

    #[test]
    #[should_panic(expected = "Insufficient funds")]
    fn test_insufficient_balance_subaccount() {
        let context = setup_context(
            "relayer.onsocial.testnet".parse().unwrap(),
            "relayer.onsocial.testnet".parse().unwrap(),
            NearToken::from_yoctonear(500_000_000_000_000_000_000), // < 0.001 NEAR + storage
            false,
        );
        testing_env!(context);
        let mut contract = OnSocialRelayer::new();
        let subaccount: AccountId = "alice.onsocial.testnet".parse().unwrap();

        contract.create_account(subaccount, VALID_PUBLIC_KEY.to_string());
    }

    #[test]
    #[should_panic(expected = "Insufficient funds")]
    fn test_insufficient_balance_toplevel() {
        let context = setup_context(
            "relayer.onsocial.testnet".parse().unwrap(),
            "relayer.onsocial.testnet".parse().unwrap(),
            NearToken::from_yoctonear(50_000_000_000_000_000_000_000), // < 0.1 NEAR + storage
            false,
        );
        testing_env!(context);
        let mut contract = OnSocialRelayer::new();
        let toplevel: AccountId = "alice.testnet".parse().unwrap();

        contract.create_top_level_account(toplevel, VALID_PUBLIC_KEY.to_string());
    }

    #[test]
    fn test_set_subaccount_balance() {
        let context = setup_context(
            "relayer.onsocial.testnet".parse().unwrap(),
            "relayer.onsocial.testnet".parse().unwrap(),
            NearToken::from_near(5),
            false,
        );
        testing_env!(context);
        let mut contract = OnSocialRelayer::new();

        contract.set_subaccount_balance(U128(2_000_000_000_000_000_000_000));
        assert_eq!(contract.get_subaccount_balance().0, 2_000_000_000_000_000_000_000);
    }

    #[test]
    #[should_panic(expected = "Only owner can perform this action")]
    fn test_set_subaccount_balance_not_owner() {
        let init_context = setup_context(
            "relayer.onsocial.testnet".parse().unwrap(),
            "relayer.onsocial.testnet".parse().unwrap(),
            NearToken::from_near(5),
            false,
        );
        testing_env!(init_context);
        let mut contract = OnSocialRelayer::new();
        assert_eq!(contract.get_owner(), "relayer.onsocial.testnet".parse::<AccountId>().unwrap());

        let call_context = setup_context(
            accounts(1),
            "relayer.onsocial.testnet".parse().unwrap(),
            NearToken::from_near(5),
            false,
        );
        testing_env!(call_context);
        contract.set_subaccount_balance(U128(2_000_000_000_000_000_000_000));
    }

    #[test]
    #[should_panic(expected = "Sub-account balance must be at least")]
    fn test_set_subaccount_balance_too_low() {
        let context = setup_context(
            "relayer.onsocial.testnet".parse().unwrap(),
            "relayer.onsocial.testnet".parse().unwrap(),
            NearToken::from_near(5),
            false,
        );
        testing_env!(context);
        let mut contract = OnSocialRelayer::new();

        contract.set_subaccount_balance(U128(500_000_000_000_000_000_000));
    }

    #[test]
    fn test_set_toplevel_balance() {
        let context = setup_context(
            "relayer.onsocial.testnet".parse().unwrap(),
            "relayer.onsocial.testnet".parse().unwrap(),
            NearToken::from_near(5),
            false,
        );
        testing_env!(context);
        let mut contract = OnSocialRelayer::new();

        contract.set_toplevel_balance(U128(200_000_000_000_000_000_000_000));
        assert_eq!(contract.get_toplevel_balance().0, 200_000_000_000_000_000_000_000);
    }

    #[test]
    #[should_panic(expected = "Only owner can perform this action")]
    fn test_set_toplevel_balance_not_owner() {
        let init_context = setup_context(
            "relayer.onsocial.testnet".parse().unwrap(),
            "relayer.onsocial.testnet".parse().unwrap(),
            NearToken::from_near(5),
            false,
        );
        testing_env!(init_context);
        let mut contract = OnSocialRelayer::new();
        assert_eq!(contract.get_owner(), "relayer.onsocial.testnet".parse::<AccountId>().unwrap());

        let call_context = setup_context(
            accounts(1),
            "relayer.onsocial.testnet".parse().unwrap(),
            NearToken::from_near(5),
            false,
        );
        testing_env!(call_context);
        contract.set_toplevel_balance(U128(200_000_000_000_000_000_000_000));
    }

    #[test]
    #[should_panic(expected = "Top-level balance must be at least")]
    fn test_set_toplevel_balance_too_low() {
        let context = setup_context(
            "relayer.onsocial.testnet".parse().unwrap(),
            "relayer.onsocial.testnet".parse().unwrap(),
            NearToken::from_near(5),
            false,
        );
        testing_env!(context);
        let mut contract = OnSocialRelayer::new();

        contract.set_toplevel_balance(U128(50_000_000_000_000_000_000_000));
    }

    #[test]
    fn test_concurrent_creations_subaccounts() {
        let context = setup_context(
            "relayer.onsocial.testnet".parse().unwrap(),
            "relayer.onsocial.testnet".parse().unwrap(),
            NearToken::from_near(5),
            false,
        );
        testing_env!(context);
        let mut contract = OnSocialRelayer::new();
        let sub1: AccountId = "bob.onsocial.testnet".parse().unwrap();
        let sub2: AccountId = "alice.onsocial.testnet".parse().unwrap();

        contract.create_account(sub1.clone(), VALID_PUBLIC_KEY.to_string());
        contract.create_account(sub2.clone(), VALID_PUBLIC_KEY.to_string());
        assert!(!contract.has_created_account(sub1));
        assert!(!contract.has_created_account(sub2));
    }

    #[test]
    fn test_concurrent_creations_toplevel() {
        let context = setup_context(
            "relayer.onsocial.testnet".parse().unwrap(),
            "relayer.onsocial.testnet".parse().unwrap(),
            NearToken::from_near(5),
            false,
        );
        testing_env!(context);
        let mut contract = OnSocialRelayer::new();
        let top1: AccountId = "bob.testnet".parse().unwrap();
        let top2: AccountId = "alice.testnet".parse().unwrap();

        contract.create_top_level_account(top1.clone(), VALID_PUBLIC_KEY.to_string());
        contract.create_top_level_account(top2.clone(), VALID_PUBLIC_KEY.to_string());
        assert!(!contract.has_created_account(top1));
        assert!(!contract.has_created_account(top2));
    }

    #[test]
    #[should_panic(expected = "Account alice.onsocial.testnet already created at block height 100")]
    fn test_duplicate_subaccount_after_creation() {
        let context = setup_context(
            "relayer.onsocial.testnet".parse().unwrap(),
            "relayer.onsocial.testnet".parse().unwrap(),
            NearToken::from_near(5),
            false,
        );
        testing_env!(context);
        let mut contract = OnSocialRelayer::new();
        let subaccount: AccountId = "alice.onsocial.testnet".parse().unwrap();

        contract.created_accounts.insert(&subaccount, &100);
        contract.create_account(subaccount, VALID_PUBLIC_KEY.to_string());
    }

    #[test]
    #[should_panic(expected = "Account alice.testnet already created at block height 100")]
    fn test_duplicate_toplevel_after_creation() {
        let context = setup_context(
            "relayer.onsocial.testnet".parse().unwrap(),
            "relayer.onsocial.testnet".parse().unwrap(),
            NearToken::from_near(5),
            false,
        );
        testing_env!(context);
        let mut contract = OnSocialRelayer::new();
        let toplevel: AccountId = "alice.testnet".parse().unwrap();

        contract.created_accounts.insert(&toplevel, &100);
        contract.create_top_level_account(toplevel, VALID_PUBLIC_KEY.to_string());
    }

    #[test]
    fn test_duplicate_subaccount_pending() {
        let context = setup_context(
            "relayer.onsocial.testnet".parse().unwrap(),
            "relayer.onsocial.testnet".parse().unwrap(),
            NearToken::from_near(5),
            false,
        );
        testing_env!(context);
        let mut contract = OnSocialRelayer::new();
        let subaccount: AccountId = "alice.onsocial.testnet".parse().unwrap();

        contract.create_account(subaccount.clone(), VALID_PUBLIC_KEY.to_string());
        contract.create_account(subaccount.clone(), VALID_PUBLIC_KEY.to_string());
        assert!(!contract.has_created_account(subaccount));
    }

    #[test]
    fn test_duplicate_toplevel_pending() {
        let context = setup_context(
            "relayer.onsocial.testnet".parse().unwrap(),
            "relayer.onsocial.testnet".parse().unwrap(),
            NearToken::from_near(5),
            false,
        );
        testing_env!(context);
        let mut contract = OnSocialRelayer::new();
        let toplevel: AccountId = "alice.testnet".parse().unwrap();

        contract.create_top_level_account(toplevel.clone(), VALID_PUBLIC_KEY.to_string());
        contract.create_top_level_account(toplevel.clone(), VALID_PUBLIC_KEY.to_string());
        assert!(!contract.has_created_account(toplevel));
    }

    #[test]
    fn test_get_contract_balance() {
        let context = setup_context(
            "relayer.onsocial.testnet".parse().unwrap(),
            "relayer.onsocial.testnet".parse().unwrap(),
            NearToken::from_near(5),
            false,
        );
        testing_env!(context);
        let contract = OnSocialRelayer::new();
        assert_eq!(contract.get_contract_balance().0, NearToken::from_near(5).as_yoctonear());
    }

    #[test]
    fn test_migration() {
        let context = setup_context(
            "relayer.onsocial.testnet".parse().unwrap(),
            "relayer.onsocial.testnet".parse().unwrap(),
            NearToken::from_near(5),
            false,
        );
        testing_env!(context);
        let mut contract = OnSocialRelayer::new();
        let subaccount: AccountId = "alice.onsocial.testnet".parse().unwrap();
        let toplevel: AccountId = "alice.testnet".parse().unwrap();
        contract.created_accounts.insert(&subaccount, &100);
        contract.created_accounts.insert(&toplevel, &100);

        let state = borsh::to_vec(&contract).unwrap();
        let old: OnSocialRelayer = BorshDeserialize::try_from_slice(&state).unwrap();
        let migrated = OnSocialRelayer {
            created_accounts: old.created_accounts,
            subaccount_balance: old.subaccount_balance,
            toplevel_balance: old.toplevel_balance,
            owner: old.owner,
        };

        assert!(migrated.has_created_account(subaccount));
        assert!(migrated.has_created_account(toplevel));
        assert_eq!(migrated.get_owner(), "relayer.onsocial.testnet".parse::<AccountId>().unwrap());
    }

    #[test]
    fn test_create_subaccount_sponsored_on_near() {
        let context = setup_context(
            "relayer.onsocial.near".parse().unwrap(),
            "relayer.onsocial.near".parse().unwrap(),
            NearToken::from_near(5),
            false,
        );
        testing_env!(context);
        let mut contract = OnSocialRelayer::new();
        let subaccount: AccountId = "alice.onsocial.near".parse().unwrap();

        contract.create_account(subaccount.clone(), VALID_PUBLIC_KEY.to_string());
        assert!(!contract.has_created_account(subaccount)); // Still pending
        // Note: Can't assert transfer amount directly in tests without mocking Promise
    }
}