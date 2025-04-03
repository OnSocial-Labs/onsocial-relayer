use near_sdk::{testing_env, test_utils::{VMContextBuilder, accounts}};
#[allow(unused_imports)] // Suppress Gas warning
use near_sdk::{AccountId, PublicKey, NearToken, Gas};
use crate::{OnSocialRelayer};
use crate::errors::RelayerError;
use crate::types::{SignedDelegateAction, DelegateAction, Action};

// Helper function to set up the contract and context
fn setup_contract() -> (OnSocialRelayer, VMContextBuilder) {
    let mut context = VMContextBuilder::new();
    context
        .predecessor_account_id(accounts(0)) // Admin account
        .attached_deposit(NearToken::from_near(0))
        .is_view(false);
    testing_env!(context.build());

    let admins = vec![accounts(0)];
    let initial_auth_account = accounts(1);
    let initial_auth_key: PublicKey = "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C19WVZw4BCrhmgbQHh".parse().unwrap();
    let offload_recipient = accounts(2);

    let contract = OnSocialRelayer::new(admins, initial_auth_account, initial_auth_key, offload_recipient);
    (contract, context)
}

// Helper to create a signed delegate action
fn create_signed_delegate(sender_id: AccountId, receiver_id: AccountId, actions: Vec<Action>) -> SignedDelegateAction {
    SignedDelegateAction {
        delegate_action: DelegateAction {
            sender_id,
            receiver_id,
            actions,
            nonce: 1,
            max_block_height: 1000,
        },
        signature: vec![0; 64], // Dummy signature (64 bytes for ed25519)
        public_key: "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C19WVZw4BCrhmgbQHh".parse().unwrap(),
        session_nonce: 1,
    }
}

// Admin Tests
#[test]
fn test_add_auth_account_success() {
    let (mut contract, mut _context) = setup_contract();
    let new_auth_account = accounts(3);
    let new_auth_key: PublicKey = "ed25519:8fWHEecB2iXjZ75kMYG34M2DSELK9nQ31K3vQ3Wy4nqW".parse().unwrap();

    let result = contract.add_auth_account(new_auth_account.clone(), new_auth_key.clone());
    assert!(result.is_ok(), "Adding auth account should succeed");
    assert_eq!(
        contract.relayer.auth_accounts.get(&new_auth_account),
        Some(&new_auth_key),
        "Auth account should be in the map"
    );
}

#[test]
fn test_add_auth_account_unauthorized() {
    let (mut contract, mut context) = setup_contract();
    context.predecessor_account_id(accounts(3)); // Non-admin
    testing_env!(context.build());

    let new_auth_account = accounts(4);
    let new_auth_key: PublicKey = "ed25519:8fWHEecB2iXjZ75kMYG34M2DSELK9nQ31K3vQ3Wy4nqW".parse().unwrap();

    let result = contract.add_auth_account(new_auth_account, new_auth_key);
    assert!(matches!(result, Err(RelayerError::Unauthorized)), "Non-admin should not add auth account");
}

#[test]
fn test_remove_auth_account_success() {
    let (mut contract, _) = setup_contract();
    let auth_account = accounts(1); // Initial auth account

    let result = contract.remove_auth_account(auth_account.clone());
    assert!(result.is_ok(), "Removing auth account should succeed");
    assert!(contract.relayer.auth_accounts.get(&auth_account).is_none(), "Auth account should be removed");
}

#[test]
fn test_set_offload_recipient_success() {
    let (mut contract, _context) = setup_contract();
    let new_recipient = accounts(3);

    let result = contract.set_offload_recipient(new_recipient.clone());
    assert!(result.is_ok(), "Setting offload recipient should succeed");
    assert_eq!(contract.relayer.offload_recipient, new_recipient, "Offload recipient should be updated");
}

#[test]
fn test_add_admin_success() {
    let (mut contract, _context) = setup_contract();
    let new_admin = accounts(3);

    let result = contract.add_admin(new_admin.clone());
    assert!(result.is_ok(), "Adding admin should succeed");
    assert!(contract.relayer.admins.contains(&new_admin), "New admin should be in the list");
    assert_eq!(contract.relayer.admins.len(), 2, "Admin list should have 2 entries");
}

#[test]
fn test_add_admin_unauthorized() {
    let (mut contract, mut context) = setup_contract();
    context.predecessor_account_id(accounts(3)); // Non-admin
    testing_env!(context.build());

    let new_admin = accounts(4);

    let result = contract.add_admin(new_admin);
    assert!(matches!(result, Err(RelayerError::Unauthorized)), "Non-admin should not add admin");
}

#[test]
fn test_add_admin_duplicate() {
    let (mut contract, _context) = setup_contract();
    let existing_admin = accounts(0); // Already an admin

    let result = contract.add_admin(existing_admin.clone());
    assert!(result.is_ok(), "Adding duplicate admin should succeed but not change state");
    assert_eq!(contract.relayer.admins.len(), 1, "Admin list should not grow with duplicate");
}

#[test]
fn test_remove_admin_success() {
    let (mut contract, _context) = setup_contract();
    // First, add a second admin so we can remove one
    let new_admin = accounts(3);
    contract.add_admin(new_admin.clone()).unwrap();

    let result = contract.remove_admin(new_admin.clone());
    assert!(result.is_ok(), "Removing admin should succeed");
    assert!(!contract.relayer.admins.contains(&new_admin), "Admin should be removed");
    assert_eq!(contract.relayer.admins.len(), 1, "Admin list should have 1 entry");
}

#[test]
fn test_remove_admin_last_admin() {
    let (mut contract, _context) = setup_contract();
    let last_admin = accounts(0); // Only admin

    let result = contract.remove_admin(last_admin);
    assert!(matches!(result, Err(RelayerError::LastAdmin)), "Should fail when removing last admin");
    assert_eq!(contract.relayer.admins.len(), 1, "Admin list should remain unchanged");
}

#[test]
fn test_remove_admin_unauthorized() {
    let (mut contract, mut context) = setup_contract();
    context.predecessor_account_id(accounts(3)); // Non-admin
    testing_env!(context.build());

    let admin_to_remove = accounts(0);

    let result = contract.remove_admin(admin_to_remove);
    assert!(matches!(result, Err(RelayerError::Unauthorized)), "Non-admin should not remove admin");
}

#[test]
fn test_set_sponsor_amount_success() {
    let (mut contract, _context) = setup_contract();
    let new_amount = 200_000_000_000_000_000_000_000; // 0.2 NEAR

    let result = contract.set_sponsor_amount(new_amount.into());
    assert!(result.is_ok(), "Setting sponsor amount should succeed");
    assert_eq!(contract.relayer.sponsor_amount, new_amount, "Sponsor amount should be updated");
}

#[test]
fn test_set_sponsor_amount_too_low() {
    let (mut contract, _context) = setup_contract();
    let too_low_amount = 5_000_000_000_000_000_000_000; // 0.005 NEAR, below 0.01 NEAR threshold

    let result = contract.set_sponsor_amount(too_low_amount.into());
    assert!(matches!(result, Err(RelayerError::AmountTooLow)), "Should fail when amount too low");
    assert_eq!(contract.relayer.sponsor_amount, 100_000_000_000_000_000_000_000, "Sponsor amount should not change");
}

#[test]
fn test_set_sponsor_amount_unauthorized() {
    let (mut contract, mut context) = setup_contract();
    context.predecessor_account_id(accounts(3)); // Non-admin
    testing_env!(context.build());

    let new_amount = 200_000_000_000_000_000_000_000; // 0.2 NEAR

    let result = contract.set_sponsor_amount(new_amount.into());
    assert!(matches!(result, Err(RelayerError::Unauthorized)), "Non-admin should not set sponsor amount");
}

// Gas Pool Tests
#[test]
fn test_deposit_gas_pool_success() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_near(2));
    testing_env!(context.build());

    let result = contract.deposit_gas_pool();
    assert!(result.is_ok(), "Deposit should succeed");
    assert_eq!(contract.get_gas_pool().0, 2_000_000_000_000_000_000_000_000, "Gas pool should increase by 2 NEAR");
}

#[test]
fn test_deposit_gas_pool_excess() {
    let (mut contract, mut context) = setup_contract();
    let max_gas_pool = contract.relayer.max_gas_pool;
    context.attached_deposit(NearToken::from_near(600)); // Exceeds 500 NEAR max
    testing_env!(context.build());

    let result = contract.deposit_gas_pool();
    assert!(result.is_ok(), "Deposit should succeed");
    assert_eq!(contract.get_gas_pool().0, max_gas_pool, "Gas pool should cap at max");
}

// Relay Tests
#[test]
fn test_relay_meta_transaction_success() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_near(10));
    testing_env!(context.build());
    contract.deposit_gas_pool().unwrap();

    let signed_delegate = create_signed_delegate(
        accounts(1), // Authorized account
        accounts(2),
        vec![Action::Transfer { deposit: NearToken::from_near(1) }],
    );

    let result = contract.relay_meta_transaction(signed_delegate);
    assert!(result.is_ok(), "Relay should succeed");
    assert!(contract.get_gas_pool().0 < 10_000_000_000_000_000_000_000_000, "Gas pool should decrease");
}

#[test]
fn test_relay_meta_transaction_insufficient_gas() {
    let (mut contract, _) = setup_contract(); // No deposit, gas pool = 0
    let signed_delegate = create_signed_delegate(
        accounts(1),
        accounts(2),
        vec![Action::Transfer { deposit: NearToken::from_near(1) }],
    );

    let result = contract.relay_meta_transaction(signed_delegate);
    assert!(matches!(result, Err(RelayerError::InsufficientGasPool)), "Should fail due to insufficient gas");
}

#[test]
fn test_relay_meta_transaction_unauthorized() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_near(10));
    testing_env!(context.build());
    contract.deposit_gas_pool().unwrap();

    let signed_delegate = create_signed_delegate(
        accounts(3), // Unauthorized account
        accounts(2),
        vec![Action::Transfer { deposit: NearToken::from_near(1) }],
    );

    let result = contract.relay_meta_transaction(signed_delegate);
    assert!(matches!(result, Err(RelayerError::Unauthorized)), "Should fail due to unauthorized sender");
}

#[test]
fn test_relay_meta_transactions_batch() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_near(20));
    testing_env!(context.build());
    contract.deposit_gas_pool().unwrap();

    let signed_delegates = vec![
        create_signed_delegate(accounts(1), accounts(2), vec![Action::Transfer { deposit: NearToken::from_near(1) }]),
        create_signed_delegate(accounts(1), accounts(3), vec![Action::Transfer { deposit: NearToken::from_near(1) }]),
    ];

    let result = contract.relay_meta_transactions(signed_delegates);
    assert!(result.is_ok(), "Batch relay should succeed");
    assert_eq!(result.unwrap().len(), 2, "Should return two promises");
}

// Sponsor Tests
#[test]
fn test_sponsor_account_success() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_near(10));
    testing_env!(context.build());
    contract.deposit_gas_pool().unwrap();

    let new_account_name = "testuser".to_string();
    let public_key: PublicKey = "ed25519:8fWHEecB2iXjZ75kMYG34M2DSELK9nQ31K3vQ3Wy4nqW".parse().unwrap();

    let result = contract.sponsor_account(new_account_name, public_key);
    assert!(result.is_ok(), "Sponsoring account should succeed");
    let sponsor_amount = contract.relayer.sponsor_amount;
    assert_eq!(
        contract.get_gas_pool().0,
        10_000_000_000_000_000_000_000_000 - sponsor_amount,
        "Gas pool should decrease by sponsor amount"
    );
}

#[test]
fn test_sponsor_account_insufficient_gas() {
    let (mut contract, _) = setup_contract(); // No deposit
    let new_account_name = "testuser".to_string();
    let public_key: PublicKey = "ed25519:8fWHEecB2iXjZ75kMYG34M2DSELK9nQ31K3vQ3Wy4nqW".parse().unwrap();

    let result = contract.sponsor_account(new_account_name, public_key);
    assert!(matches!(result, Err(RelayerError::InsufficientGasPool)), "Should fail due to insufficient gas");
}

// View Method Tests
#[test]
fn test_get_gas_pool() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_near(5));
    testing_env!(context.build());
    contract.deposit_gas_pool().unwrap();

    assert_eq!(contract.get_gas_pool().0, 5_000_000_000_000_000_000_000_000, "Should return correct gas pool amount");
}

#[test]
fn test_get_min_gas_pool() {
    let (contract, _) = setup_contract();
    assert_eq!(contract.get_min_gas_pool().0, 1_000_000_000_000_000_000_000_000, "Should return min gas pool");
}

#[test]
fn test_get_sponsor_amount() {
    let (contract, _) = setup_contract();
    assert_eq!(contract.get_sponsor_amount().0, 100_000_000_000_000_000_000_000, "Should return sponsor amount");
}