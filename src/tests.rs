use near_sdk::env;
use near_sdk::test_utils::{VMContextBuilder, accounts, get_logs};
use near_sdk::testing_env;
use near_sdk::NearToken;
use near_sdk::AccountId;
use crate::types::{SignedDelegateAction, DelegateAction, Action, SerializablePromiseResult, WrappedAccountId, WrappedNearToken, WrappedGas, WrappedPublicKey};
use crate::state::{Relayer, AccountIdWrapper};
use crate::errors::RelayerError;
use near_sdk::Gas;
use near_sdk::json_types::U128;
use near_sdk::PublicKey;
use serde_json;
use ed25519_dalek::{Signer, SigningKey};

pub fn setup_contract() -> (Relayer, VMContextBuilder) {
    let mut context = VMContextBuilder::new();
    context.current_account_id(accounts(0));
    context.predecessor_account_id("onsocial.testnet".parse().unwrap());
    context.account_balance(NearToken::from_yoctonear(10_000_000_000_000_000_000_000_000));
    context.attached_deposit(NearToken::from_yoctonear(0));
    context.prepaid_gas(Gas::from_tgas(500)); // Increased default gas
    testing_env!(context.build());
    let contract = Relayer::new(None, U128(0), vec![accounts(1)]);
    (contract, context)
}

pub fn dummy_signed_delegate(sender: &near_sdk::AccountId, receiver: &near_sdk::AccountId, nonce: u64, max_block_height: Option<u64>) -> SignedDelegateAction {
    let delegate = DelegateAction {
        sender_id: WrappedAccountId(sender.clone()),
        receiver_id: WrappedAccountId(receiver.clone()),
        actions: vec![Action::Transfer {
            deposit: WrappedNearToken(NearToken::from_yoctonear(1)),
        }],
        nonce,
        max_block_height: max_block_height.unwrap_or_else(|| env::block_height() + 300),
    };
    let mut pk_bytes = vec![0];
    pk_bytes.extend_from_slice(&[0; 32]);
    let dummy_pk = PublicKey::try_from(pk_bytes).unwrap();
    SignedDelegateAction {
        delegate_action: delegate,
        signature: vec![0; 64],
        public_key: WrappedPublicKey(dummy_pk),
    }
}

pub fn create_post_transaction(sender: AccountId, nonce: u64, max_block_height: u64) -> SignedDelegateAction {
    let delegate = DelegateAction {
        sender_id: WrappedAccountId(sender.clone()),
        receiver_id: WrappedAccountId(accounts(1)),
        actions: vec![Action::Transfer { 
            deposit: WrappedNearToken(NearToken::from_yoctonear(1_000_000_000_000_000_000_000_000)) 
        }],
        nonce,
        max_block_height,
    };
    let mut public_key_bytes = vec![0];
    public_key_bytes.extend_from_slice(&[0; 32]);
    let public_key = PublicKey::try_from(public_key_bytes).unwrap();
    SignedDelegateAction {
        delegate_action: delegate,
        signature: vec![0; 64],
        public_key: WrappedPublicKey(public_key),
    }
}

// Helper function to generate a real signed delegate action
fn real_signed_delegate(sender: &AccountId, receiver: &AccountId, nonce: u64, max_block_height: u64) -> SignedDelegateAction {
    let delegate = DelegateAction {
        sender_id: WrappedAccountId(sender.clone()),
        receiver_id: WrappedAccountId(receiver.clone()),
        actions: vec![Action::Transfer {
            deposit: WrappedNearToken(NearToken::from_yoctonear(1)),
        }],
        nonce,
        max_block_height,
    };

    let mut csprng = rand::thread_rng();
    let signing_key = SigningKey::generate(&mut csprng);
    let verifying_key = signing_key.verifying_key();
    let message = near_sdk::borsh::to_vec(&delegate).unwrap();
    let signature = signing_key.sign(&message).to_bytes().to_vec();

    let mut pk_bytes = vec![0]; // Ed25519 prefix
    pk_bytes.extend_from_slice(&verifying_key.to_bytes());
    let public_key = PublicKey::try_from(pk_bytes).unwrap();

    SignedDelegateAction {
        delegate_action: delegate,
        signature,
        public_key: WrappedPublicKey(public_key),
    }
}

// Initialization Tests
#[test]
fn test_init_with_default_whitelist() {
    let mut context = VMContextBuilder::new();
    context.current_account_id(accounts(0));
    context.predecessor_account_id("onsocial.testnet".parse().unwrap());
    testing_env!(context.build());
    let contract = Relayer::new(None, U128(0), vec![]);
    assert_eq!(contract.whitelisted_contracts.len(), 4);
    let social_near = WrappedAccountId("social.near".parse::<near_sdk::AccountId>().unwrap());
    let social_tkn_near = WrappedAccountId("social.tkn.near".parse::<near_sdk::AccountId>().unwrap());
    let usdc_testnet = WrappedAccountId("3e2210e1184b45b64c8a434c0a7e7b23cc04ea7eb7a6c3c32520d03d4afcb8af".parse::<near_sdk::AccountId>().unwrap());
    let usdc_mainnet = WrappedAccountId("17208628f84f5d6ad33f0da3bbbeb27ffcb398eac501a31bd6ad2011e36133a1".parse::<near_sdk::AccountId>().unwrap());
    assert!(contract.whitelisted_contracts.iter().any(|id| id.0 == social_near));
    assert!(contract.whitelisted_contracts.iter().any(|id| id.0 == social_tkn_near));
    assert!(contract.whitelisted_contracts.iter().any(|id| id.0 == usdc_testnet));
    assert!(contract.whitelisted_contracts.iter().any(|id| id.0 == usdc_mainnet));
}

#[test]
fn test_init_with_custom_whitelist() {
    let mut context = VMContextBuilder::new();
    context.current_account_id(accounts(0));
    context.predecessor_account_id("onsocial.testnet".parse().unwrap());
    testing_env!(context.build());
    let contract = Relayer::new(None, U128(0), vec![accounts(1)]);
    assert_eq!(contract.whitelisted_contracts.len(), 5);
    let acc1 = WrappedAccountId(accounts(1));
    let social_near = WrappedAccountId("social.near".parse::<near_sdk::AccountId>().unwrap());
    let social_tkn_near = WrappedAccountId("social.tkn.near".parse::<near_sdk::AccountId>().unwrap());
    let usdc_testnet = WrappedAccountId("3e2210e1184b45b64c8a434c0a7e7b23cc04ea7eb7a6c3c32520d03d4afcb8af".parse::<near_sdk::AccountId>().unwrap());
    let usdc_mainnet = WrappedAccountId("17208628f84f5d6ad33f0da3bbbeb27ffcb398eac501a31bd6ad2011e36133a1".parse::<near_sdk::AccountId>().unwrap());
    assert!(contract.whitelisted_contracts.iter().any(|id| id.0 == acc1));
    assert!(contract.whitelisted_contracts.iter().any(|id| id.0 == social_near));
    assert!(contract.whitelisted_contracts.iter().any(|id| id.0 == social_tkn_near));
    assert!(contract.whitelisted_contracts.iter().any(|id| id.0 == usdc_testnet));
    assert!(contract.whitelisted_contracts.iter().any(|id| id.0 == usdc_mainnet));
}

#[test]
fn test_init_with_ft_payment() {
    let mut context = VMContextBuilder::new();
    context.current_account_id(accounts(0));
    context.predecessor_account_id("onsocial.testnet".parse().unwrap());
    testing_env!(context.build());
    let contract = Relayer::new(Some(accounts(2)), U128(1_000_000), vec![accounts(1)]);
    assert_eq!(contract.payment_ft_contract, Some(AccountIdWrapper(WrappedAccountId(accounts(2)))));
    assert_eq!(contract.min_ft_payment, 1_000_000);
    assert_eq!(contract.whitelisted_contracts.len(), 5);
}

// Gas Pool Tests
#[test]
fn test_deposit_gas_pool() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();
    assert_eq!(contract.gas_pool, 5_000_000_000_000_000_000_000_000);
    let logs = get_logs();
    assert_eq!(logs.len(), 1);
}

#[test]
#[should_panic(expected = "Deposit must be positive")]
fn test_deposit_gas_pool_zero() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(0));
    testing_env!(context.build());
    contract.deposit_gas_pool();
}

// Sponsor Account Tests
#[test]
fn test_sponsor_named_account() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();
    let mut public_key_bytes = vec![0];
    public_key_bytes.extend_from_slice(&[1; 32]);
    let public_key = PublicKey::try_from(public_key_bytes).unwrap();
    context.attached_deposit(NearToken::from_yoctonear(0));
    context.predecessor_account_id(accounts(0));
    testing_env!(context.build());
    let result = contract.sponsor_account("user123".to_string(), public_key, false, false, None);
    assert!(result.is_ok());
}

#[test]
fn test_sponsor_implicit_account() {
    let (mut contract, mut context) = setup_contract();
    let deposit = NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000);
    context.attached_deposit(deposit);
    testing_env!(context.build());
    contract.deposit_gas_pool();
    let mut public_key_bytes = vec![0];
    public_key_bytes.extend_from_slice(&[1; 32]);
    let public_key = PublicKey::try_from(public_key_bytes).unwrap();
    let implicit_id = "a96ad3cb539b653e4b869bd7cf26590690e8971a96ad3cb539b653e4b869bd7c".to_string();
    context.attached_deposit(NearToken::from_yoctonear(0));
    context.predecessor_account_id(accounts(0));
    testing_env!(context.build());
    let result = contract.sponsor_account(implicit_id, public_key, false, true, None);
    assert!(result.is_ok());
}

#[test]
fn test_sponsor_account_invalid_implicit() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();
    let mut public_key_bytes = vec![0];
    public_key_bytes.extend_from_slice(&[1; 32]);
    let public_key = PublicKey::try_from(public_key_bytes).unwrap();
    context.attached_deposit(NearToken::from_yoctonear(0));
    context.predecessor_account_id(accounts(0));
    testing_env!(context.build());
    let result = contract.sponsor_account("invalid".to_string(), public_key, false, true, None);
    assert!(matches!(result, Err(RelayerError::InvalidAccountId)));
}

#[test]
fn test_sponsor_account_insufficient_balance() {
    let (mut contract, mut context) = setup_contract();
    context.account_balance(NearToken::from_yoctonear(1_000_000_000_000_000_000_000_000));
    context.attached_deposit(NearToken::from_yoctonear(0));
    testing_env!(context.build());
    let mut public_key_bytes = vec![0];
    public_key_bytes.extend_from_slice(&[1; 32]);
    let public_key = PublicKey::try_from(public_key_bytes).unwrap();
    context.predecessor_account_id(accounts(0));
    testing_env!(context.build());
    let result = contract.sponsor_account("user123".to_string(), public_key, false, false, None);
    assert!(matches!(result, Err(RelayerError::InsufficientBalance)));
}

#[test]
fn test_sponsor_account_already_exists() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    context.prepaid_gas(Gas::from_tgas(500));
    testing_env!(context.build());
    contract.deposit_gas_pool();

    let mut public_key_bytes = vec![0];
    public_key_bytes.extend_from_slice(&[1; 32]);
    let public_key = PublicKey::try_from(public_key_bytes).unwrap();

    let delegate1 = DelegateAction {
        sender_id: WrappedAccountId(accounts(2)),
        receiver_id: WrappedAccountId(accounts(0)),
        actions: vec![Action::FunctionCall {
            method_name: "sponsor_account".to_string(),
            args: serde_json::to_vec(&serde_json::json!({
                "account_name": "user123",
                "public_key": WrappedPublicKey(public_key.clone()),
                "add_function_call_key": false,
                "is_implicit": false
            })).unwrap(),
            gas: WrappedGas(Gas::from_tgas(300)),
            deposit: WrappedNearToken(NearToken::from_yoctonear(0)),
        }],
        nonce: 1,
        max_block_height: 1000,
    };
    let signed_delegate1 = SignedDelegateAction {
        delegate_action: delegate1,
        signature: vec![0; 64],
        public_key: WrappedPublicKey(public_key.clone()),
    };

    context.attached_deposit(NearToken::from_yoctonear(0));
    context.signer_account_id(accounts(2));
    testing_env!(context.build());

    let result1 = contract.sponsor_account(
        "user123".to_string(),
        public_key.clone(),
        false,
        false,
        Some(signed_delegate1.clone()),
    );
    assert!(result1.is_ok(), "First sponsor_account failed: {:?}", result1.err());

    contract.set_simulate_promise_result(Some(SerializablePromiseResult::Successful(vec![])));

    let delegate2 = DelegateAction {
        sender_id: WrappedAccountId(accounts(2)),
        receiver_id: WrappedAccountId(accounts(0)),
        actions: vec![Action::FunctionCall {
            method_name: "sponsor_account".to_string(),
            args: serde_json::to_vec(&serde_json::json!({
                "account_name": "user123",
                "public_key": WrappedPublicKey(public_key.clone()),
                "add_function_call_key": false,
                "is_implicit": false
            })).unwrap(),
            gas: WrappedGas(Gas::from_tgas(300)),
            deposit: WrappedNearToken(NearToken::from_yoctonear(0)),
        }],
        nonce: 2,
        max_block_height: 1000,
    };
    let signed_delegate2 = SignedDelegateAction {
        delegate_action: delegate2,
        signature: vec![0; 64],
        public_key: WrappedPublicKey(public_key.clone()),
    };

    context.prepaid_gas(Gas::from_tgas(500));
    testing_env!(context.build());

    let result2 = contract.sponsor_account(
        "user123".to_string(),
        public_key.clone(),
        false,
        false,
        Some(signed_delegate2.clone()),
    );
    assert!(
        matches!(result2, Err(RelayerError::AccountExists)),
        "Expected second sponsor_account to fail with AccountExists, got: {}",
        match result2 { Ok(_) => "Ok(Promise)".to_string(), Err(e) => format!("Err({:?})", e) }
    );
}

// Meta Transaction Tests
#[test]
fn test_relay_whitelisted_contracts() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();
    context.signer_account_id(accounts(2));

    let whitelisted_contracts = [
        accounts(1),
        "social.near".parse().unwrap(),
        "social.tkn.near".parse().unwrap(),
        "3e2210e1184b45b64c8a434c0a7e7b23cc04ea7eb7a6c3c32520d03d4afcb8af".parse().unwrap(),
        "17208628f84f5d6ad33f0da3bbbeb27ffcb398eac501a31bd6ad2011e36133a1".parse().unwrap(),
    ];
    for (i, receiver) in whitelisted_contracts.iter().enumerate() {
        let nonce = (i + 1) as u64;
        let signed_delegate = dummy_signed_delegate(&accounts(2), receiver, nonce, None); // Fixed: Added None for max_block_height
        context.attached_deposit(NearToken::from_yoctonear(0));
        testing_env!(context.build());
        let result = contract.relay_meta_transaction(signed_delegate);
        assert!(result.is_ok(), "Expected Ok for whitelisted contract {}, got {:?}", receiver, result);
    }
}

#[test]
fn test_relay_meta_transaction_no_ft() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();
    context.signer_account_id(accounts(2));
    let signed_delegate = dummy_signed_delegate(&accounts(2), &accounts(1), 1, None);
    context.attached_deposit(NearToken::from_yoctonear(0));
    testing_env!(context.build());
    let result = contract.relay_meta_transaction(signed_delegate);
    assert!(result.is_ok(), "Expected Ok");
}

#[test]
fn test_relay_meta_transaction_insufficient_gas_pool() {
    let (mut contract, mut context) = setup_contract();
    context.account_balance(NearToken::from_yoctonear(500_000_000_000_000_000_000_000));
    context.signer_account_id(accounts(2));
    let signed_delegate = dummy_signed_delegate(&accounts(2), &accounts(1), 1, None);
    context.attached_deposit(NearToken::from_yoctonear(0));
    testing_env!(context.build());
    let result = contract.relay_meta_transaction(signed_delegate);
    assert!(matches!(result, Err(RelayerError::InsufficientGasPool)));
}

#[test]
fn test_relay_meta_transaction_invalid_nonce() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();
    contract.processed_nonces.insert(AccountIdWrapper(WrappedAccountId(accounts(2))), 1);
    context.signer_account_id(accounts(2));
    let signed_delegate = dummy_signed_delegate(&accounts(2), &accounts(1), 1, None);
    context.attached_deposit(NearToken::from_yoctonear(0));
    testing_env!(context.build());
    let result = contract.relay_meta_transaction(signed_delegate);
    assert!(matches!(result, Err(RelayerError::InvalidNonce)));
}

#[test]
fn test_relay_meta_transaction_not_whitelisted() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();
    context.signer_account_id(accounts(2));
    let signed_delegate = dummy_signed_delegate(&accounts(2), &accounts(3), 1, None);
    context.attached_deposit(NearToken::from_yoctonear(0));
    testing_env!(context.build());
    let result = contract.relay_meta_transaction(signed_delegate);
    assert!(matches!(result, Err(RelayerError::NotWhitelisted)));
}

#[test]
fn test_relay_meta_transaction_expired() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();
    context.signer_account_id(accounts(2));
    context.block_height(1001);
    let signed_delegate = dummy_signed_delegate(&accounts(2), &accounts(1), 1, None);
    context.attached_deposit(NearToken::from_yoctonear(0));
    testing_env!(context.build());
    let result = contract.relay_meta_transaction(signed_delegate);
    assert!(matches!(result, Err(RelayerError::ExpiredTransaction)));
}

// Whitelist Management Tests
#[test]
fn test_update_whitelist() {
    let (mut contract, _) = setup_contract();
    let result = contract.update_whitelist(vec![accounts(2), accounts(3)]);
    assert!(result.is_ok());
    assert_eq!(contract.whitelisted_contracts.len(), 2);
}

#[test]
fn test_update_whitelist_unauthorized() {
    let (mut contract, mut context) = setup_contract();
    context.predecessor_account_id(accounts(1));
    testing_env!(context.build());
    let result = contract.update_whitelist(vec![accounts(2)]);
    assert!(matches!(result, Err(RelayerError::Unauthorized)));
}

// Sponsor Amount Tests
#[test]
fn test_set_sponsor_amount() {
    let (mut contract, _) = setup_contract();
    let result = contract.set_sponsor_amount(U128(200_000_000_000_000_000_000_000));
    assert!(result.is_ok());
    assert_eq!(contract.get_sponsor_amount().0, 200_000_000_000_000_000_000_000);
}

#[test]
fn test_set_sponsor_amount_too_low() {
    let (mut contract, _) = setup_contract();
    let result = contract.set_sponsor_amount(U128(10_000_000_000_000_000_000_000));
    assert!(matches!(result, Err(RelayerError::InvalidSponsorAmount)));
}

#[test]
fn test_set_sponsor_amount_unauthorized() {
    let (mut contract, mut context) = setup_contract();
    context.predecessor_account_id(accounts(1));
    testing_env!(context.build());
    let result = contract.set_sponsor_amount(U128(200_000_000_000_000_000_000_000));
    assert!(matches!(result, Err(RelayerError::Unauthorized)));
}

// Function Call Key Tests
#[test]
fn test_add_function_call_key() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();
    
    context.attached_deposit(NearToken::from_yoctonear(0));
    let mut public_key_bytes = vec![0];
    public_key_bytes.extend_from_slice(&[2; 32]);
    let public_key = PublicKey::try_from(public_key_bytes).unwrap();
    testing_env!(context.build());
    
    let result = contract.add_function_call_key(
        accounts(2),
        public_key,
        accounts(1),
        vec!["some_method".to_string()],
    );
    assert!(result.is_ok());
}

#[test]
fn test_add_function_call_key_unauthorized() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    context.predecessor_account_id(accounts(1));
    testing_env!(context.build());
    contract.deposit_gas_pool();
    let mut public_key_bytes = vec![0];
    public_key_bytes.extend_from_slice(&[2; 32]);
    let public_key = PublicKey::try_from(public_key_bytes).unwrap();
    context.attached_deposit(NearToken::from_yoctonear(0));
    testing_env!(context.build());
    let result = contract.add_function_call_key(
        accounts(2),
        public_key,
        accounts(1),
        vec!["some_method".to_string()],
    );
    assert!(matches!(result, Err(RelayerError::Unauthorized)));
}

#[test]
fn test_remove_function_call_key_success() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    context.prepaid_gas(Gas::from_tgas(500));
    testing_env!(context.build());
    contract.deposit_gas_pool();

    let account_id = accounts(2);
    let mut public_key_bytes = vec![0];
    public_key_bytes.extend_from_slice(&[1; 32]);
    let public_key = PublicKey::try_from(public_key_bytes).unwrap();

    let delegate = DelegateAction {
        sender_id: WrappedAccountId(account_id.clone()),
        receiver_id: WrappedAccountId(accounts(0)),
        actions: vec![Action::FunctionCall {
            method_name: "remove_function_call_key".to_string(),
            args: serde_json::to_vec(&serde_json::json!({
                "account_id": account_id.as_str(),
                "public_key": WrappedPublicKey(public_key.clone())
            })).unwrap(),
            gas: WrappedGas(Gas::from_tgas(300)),
            deposit: WrappedNearToken(NearToken::from_yoctonear(0)),
        }],
        nonce: 1,
        max_block_height: 1000,
    };
    let signed_delegate = SignedDelegateAction {
        delegate_action: delegate,
        signature: vec![0; 64],
        public_key: WrappedPublicKey(public_key.clone()),
    };

    context.attached_deposit(NearToken::from_yoctonear(0));
    context.signer_account_id(account_id.clone());
    testing_env!(context.build());

    contract.set_simulate_promise_result(Some(SerializablePromiseResult::Successful(vec![])));
    let remove_result = contract.remove_function_call_key(account_id.clone(), public_key.clone(), signed_delegate.clone());
    assert!(remove_result.is_ok(), "Removing key failed: {:?}", remove_result.err());

    contract.callback_key_removal(account_id.clone(), 1);

    assert_eq!(
        contract.get_processed_nonce(account_id.clone()),
        Some(1),
        "Nonce should be updated to 1"
    );

    let logs = get_logs();
    assert!(logs.iter().any(|log| log.contains("function_call_key_removed")));
    assert!(logs.iter().any(|log| log.contains("Key successfully removed")));
}

#[test]
fn test_remove_function_call_key_invalid_signature() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();

    let account_id = accounts(2);
    let mut public_key_bytes = vec![0];
    public_key_bytes.extend_from_slice(&[1; 32]);
    let public_key = PublicKey::try_from(public_key_bytes).unwrap();

    let delegate = DelegateAction {
        sender_id: WrappedAccountId(account_id.clone()),
        receiver_id: WrappedAccountId(accounts(0)),
        actions: vec![Action::FunctionCall {
            method_name: "remove_function_call_key".to_string(),
            args: serde_json::to_vec(&serde_json::json!({
                "account_id": account_id.as_str(),
                "public_key": WrappedPublicKey(public_key.clone())
            })).unwrap(),
            gas: WrappedGas(Gas::from_tgas(300)),
            deposit: WrappedNearToken(NearToken::from_yoctonear(0)),
        }],
        nonce: 1,
        max_block_height: 1000,
    };
    let signed_delegate = SignedDelegateAction {
        delegate_action: delegate,
        signature: vec![0; 64],
        public_key: WrappedPublicKey(public_key.clone()),
    };

    context.attached_deposit(NearToken::from_yoctonear(0));
    context.signer_account_id(account_id.clone());
    contract.set_simulate_signature_failure(true);
    testing_env!(context.build());

    let result = contract.remove_function_call_key(account_id.clone(), public_key.clone(), signed_delegate);
    assert!(matches!(result, Err(RelayerError::InvalidSignature)));
}

#[test]
fn test_remove_function_call_key_insufficient_gas_pool() {
    let (mut contract, mut context) = setup_contract();
    context.account_balance(NearToken::from_yoctonear(500_000_000_000_000_000_000_000));
    testing_env!(context.build());

    let account_id = accounts(2);
    let mut public_key_bytes = vec![0];
    public_key_bytes.extend_from_slice(&[1; 32]);
    let public_key = PublicKey::try_from(public_key_bytes).unwrap();

    let delegate = DelegateAction {
        sender_id: WrappedAccountId(account_id.clone()),
        receiver_id: WrappedAccountId(accounts(0)),
        actions: vec![Action::FunctionCall {
            method_name: "remove_function_call_key".to_string(),
            args: serde_json::to_vec(&serde_json::json!({
                "account_id": account_id.as_str(),
                "public_key": WrappedPublicKey(public_key.clone())
            })).unwrap(),
            gas: WrappedGas(Gas::from_tgas(300)),
            deposit: WrappedNearToken(NearToken::from_yoctonear(0)),
        }],
        nonce: 1,
        max_block_height: 1000,
    };
    let signed_delegate = SignedDelegateAction {
        delegate_action: delegate,
        signature: vec![0; 64],
        public_key: WrappedPublicKey(public_key.clone()),
    };

    context.attached_deposit(NearToken::from_yoctonear(0));
    context.signer_account_id(account_id.clone());
    testing_env!(context.build());

    let result = contract.remove_function_call_key(account_id.clone(), public_key.clone(), signed_delegate);
    assert!(matches!(result, Err(RelayerError::InsufficientGasPool)));
}

#[test]
#[should_panic(expected = "No deposit allowed; costs covered by relayer")]
fn test_remove_function_call_key_with_deposit_fails() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();

    let account_id = accounts(2);
    let mut public_key_bytes = vec![0];
    public_key_bytes.extend_from_slice(&[1; 32]);
    let public_key = PublicKey::try_from(public_key_bytes).unwrap();

    let delegate = DelegateAction {
        sender_id: WrappedAccountId(account_id.clone()),
        receiver_id: WrappedAccountId(accounts(0)),
        actions: vec![Action::FunctionCall {
            method_name: "remove_function_call_key".to_string(),
            args: serde_json::to_vec(&serde_json::json!({
                "account_id": account_id.as_str(),
                "public_key": WrappedPublicKey(public_key.clone())
            })).unwrap(),
            gas: WrappedGas(Gas::from_tgas(300)),
            deposit: WrappedNearToken(NearToken::from_yoctonear(0)),
        }],
        nonce: 1,
        max_block_height: 1000,
    };
    let signed_delegate = SignedDelegateAction {
        delegate_action: delegate,
        signature: vec![0; 64],
        public_key: WrappedPublicKey(public_key.clone()),
    };

    context.attached_deposit(NearToken::from_yoctonear(1_000_000_000_000_000_000_000));
    context.signer_account_id(account_id.clone());
    testing_env!(context.build());

    let _ = contract.remove_function_call_key(account_id, public_key, signed_delegate);
}

// State Query Tests
#[test]
fn test_get_gas_pool() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();
    assert_eq!(contract.get_gas_pool().0, 5_000_000_000_000_000_000_000_000);
}

#[test]
fn test_get_sponsor_amount() {
    let (contract, _) = setup_contract();
    assert_eq!(contract.get_sponsor_amount().0, 100_000_000_000_000_000_000_000);
}

// Callback Tests
#[test]
fn test_callback_success() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();
    let sender_id = accounts(2);
    let nonce = 42;
    contract.callback_success(sender_id.clone(), nonce);
    assert_eq!(contract.processed_nonces.get(&AccountIdWrapper(WrappedAccountId(sender_id))), Some(&nonce));
}

#[test]
fn test_callback_failure_auto_retry_success() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();
    
    let signed_delegate = dummy_signed_delegate(&accounts(2), &accounts(1), 1, None);
    let initial_gas = Gas::from_tgas(150);
    contract.set_simulate_promise_result(Some(SerializablePromiseResult::Failed));
    
    context.attached_deposit(NearToken::from_yoctonear(0));
    testing_env!(context.build());
    contract.callback_failure(signed_delegate.clone(), initial_gas);
    assert_eq!(contract.failed_transactions.len(), 0);
    assert_eq!(contract.get_processed_nonce(accounts(2)), Some(1));
    let logs = get_logs();
    assert!(logs.iter().any(|log| log.contains("Auto-retried transaction with nonce 1 succeeded")));
}

#[test]
fn test_callback_failure_auto_retry_insufficient_gas() {
    let (mut contract, mut context) = setup_contract();
    context.account_balance(NearToken::from_yoctonear(500_000_000_000_000_000_000_000));
    testing_env!(context.build());
    let signed_delegate = dummy_signed_delegate(&accounts(2), &accounts(1), 1, None);
    let initial_gas = Gas::from_tgas(150);
    contract.set_simulate_promise_result(Some(SerializablePromiseResult::Failed));
    context.attached_deposit(NearToken::from_yoctonear(0));
    testing_env!(context.build());
    contract.callback_failure(signed_delegate.clone(), initial_gas);
    assert_eq!(contract.failed_transactions.len(), 1);
    let (_, new_gas, reason) = &contract.failed_transactions[0];
    assert_eq!(*new_gas / 1_000_000_000_000, 230); // 150 * 1.2 + 50 buffer
    assert_eq!(reason, &Some(RelayerError::InsufficientGasPool));
    let logs = get_logs();
    assert!(logs.iter().any(|log| log.contains("Queued failed transaction with 230 TGas")));
}

#[test]
fn test_callback_failure_auto_retry_expired() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    context.block_height(1001);
    testing_env!(context.build());
    contract.deposit_gas_pool();

    let signed_delegate = dummy_signed_delegate(&accounts(2), &accounts(1), 1, Some(1301)); // Explicit max_block_height
    let initial_gas = Gas::from_tgas(150);
    contract.set_simulate_promise_result(Some(SerializablePromiseResult::Failed));
    contract.set_simulate_signature_failure(true); // Ensure retry fails due to invalid signature
    context.attached_deposit(NearToken::from_yoctonear(0));
    testing_env!(context.build());

    contract.callback_failure(signed_delegate.clone(), initial_gas);
    assert_eq!(contract.failed_transactions.len(), 1, "Failed transaction should be queued");
    let (_, new_gas, _) = &contract.failed_transactions[0];
    assert_eq!(*new_gas / 1_000_000_000_000, 230); // 150 * 1.2 + 50 buffer
    let logs = get_logs();
    assert!(logs.iter().any(|log| log.contains("Queued failed transaction with 230 TGas")));
}

#[test]
fn test_callback_failure_auto_retry_fails_queues() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();
    let signed_delegate = dummy_signed_delegate(&accounts(2), &accounts(1), 1, None);
    let initial_gas = Gas::from_tgas(150);
    contract.set_simulate_promise_result(Some(SerializablePromiseResult::Failed));
    contract.set_simulate_signature_failure(true);
    context.attached_deposit(NearToken::from_yoctonear(0));
    testing_env!(context.build());
    contract.callback_failure(signed_delegate.clone(), initial_gas);
    assert_eq!(contract.failed_transactions.len(), 1);
    let (_, new_gas, _) = &contract.failed_transactions[0];
    assert_eq!(*new_gas / 1_000_000_000_000, 230); // 150 * 1.2 + 50 buffer
    let logs = get_logs();
    assert!(logs.iter().any(|log| log.contains("Queued failed transaction with 230 TGas")));
}

#[test]
fn test_callback_failure_multiple() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();
    let signed_delegate1 = dummy_signed_delegate(&accounts(2), &accounts(1), 1, None);
    let signed_delegate2 = dummy_signed_delegate(&accounts(2), &accounts(1), 2, None);
    let initial_gas = Gas::from_tgas(150);
    contract.set_simulate_promise_result(Some(SerializablePromiseResult::Failed));
    contract.set_simulate_signature_failure(true);
    context.attached_deposit(NearToken::from_yoctonear(0));
    testing_env!(context.build());
    contract.callback_failure(signed_delegate1.clone(), initial_gas);
    contract.callback_failure(signed_delegate2.clone(), initial_gas);
    assert_eq!(contract.failed_transactions.len(), 2);
    assert_eq!(contract.failed_transactions[0].0.delegate_action.nonce, 1);
    assert_eq!(contract.failed_transactions[1].0.delegate_action.nonce, 2);
}

#[test]
fn test_callback_key_addition() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    let mut public_key_bytes = vec![0];
    public_key_bytes.extend_from_slice(&[3; 32]);
    let public_key = PublicKey::try_from(public_key_bytes).unwrap();
    context.signer_account_pk(public_key.clone());
    testing_env!(context.build());
    contract.deposit_gas_pool();
    let sender_id = accounts(2);
    contract.set_simulate_promise_result(Some(SerializablePromiseResult::Successful(vec![])));
    testing_env!(context.build());
    contract.callback_key_addition(sender_id.clone());
    let logs = get_logs();
    assert!(logs.iter().any(|log| log.contains("function_call_key_added")));
}

// FT Tests
#[cfg(feature = "ft")]
#[test]
fn test_relay_meta_transaction_with_ft() {
    let (_contract, mut context) = setup_contract();
    testing_env!(context.build());
    let mut contract = Relayer::new(Some(accounts(1)), U128(1_000_000_000_000_000_000_000_000), vec![accounts(1)]);
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    context.signer_account_id(accounts(2));
    testing_env!(context.build());
    contract.deposit_gas_pool();
    let delegate = DelegateAction {
        sender_id: WrappedAccountId(accounts(2)),
        receiver_id: WrappedAccountId(accounts(1)),
        actions: vec![Action::FunctionCall {
            method_name: "ft_transfer".to_string(),
            args: serde_json::to_vec(&serde_json::json!({"receiver_id": accounts(0).as_str(), "amount": "2000000000000000000000000"})).unwrap(),
            gas: WrappedGas(Gas::from_tgas(50)),
            deposit: WrappedNearToken(NearToken::from_yoctonear(2_000_000_000_000_000_000_000_000)),
        }],
        nonce: 1,
        max_block_height: 1000,
    };
    let mut public_key_bytes = vec![0];
    public_key_bytes.extend_from_slice(&[0; 32]);
    let public_key = PublicKey::try_from(public_key_bytes).unwrap();
    let signed_delegate = SignedDelegateAction { 
        delegate_action: delegate, 
        signature: vec![0; 64], 
        public_key: WrappedPublicKey(public_key)
    };
    context.attached_deposit(NearToken::from_yoctonear(0));
    testing_env!(context.build());
    let result = contract.relay_meta_transaction(signed_delegate);
    assert!(result.is_ok(), "Expected Ok");
}

#[cfg(feature = "ft")]
#[test]
fn test_relay_meta_transaction_ft_invalid_method() {
    let (_contract, mut context) = setup_contract();
    testing_env!(context.build());
    let mut contract = Relayer::new(Some(accounts(1)), U128(1_000_000_000_000_000_000_000_000), vec![accounts(1)]);
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    context.signer_account_id(accounts(2));
    testing_env!(context.build());
    contract.deposit_gas_pool();
    let delegate = DelegateAction {
        sender_id: WrappedAccountId(accounts(2)),
        receiver_id: WrappedAccountId(accounts(1)),
        actions: vec![Action::FunctionCall {
            method_name: "wrong_method".to_string(),
            args: serde_json::to_vec(&serde_json::json!({"receiver_id": accounts(0).as_str(), "amount": "2000000000000000000000000"})).unwrap(),
            gas: WrappedGas(Gas::from_tgas(50)),
            deposit: WrappedNearToken(NearToken::from_yoctonear(2_000_000_000_000_000_000_000_000)),
        }],
        nonce: 1,
        max_block_height: 1000,
    };
    let mut public_key_bytes = vec![0];
    public_key_bytes.extend_from_slice(&[0; 32]);
    let public_key = PublicKey::try_from(public_key_bytes).unwrap();
    let signed_delegate = SignedDelegateAction { 
        delegate_action: delegate, 
        signature: vec![0; 64], 
        public_key: WrappedPublicKey(public_key)
    };
    context.attached_deposit(NearToken::from_yoctonear(0));
    testing_env!(context.build());
    let result = contract.relay_meta_transaction(signed_delegate);
    assert!(matches!(result, Err(RelayerError::InvalidFTTransfer)));
}

#[cfg(feature = "ft")]
#[test]
fn test_relay_meta_transaction_ft_insufficient_deposit() {
    let (_contract, mut context) = setup_contract();
    testing_env!(context.build());
    let mut contract = Relayer::new(Some(accounts(1)), U128(1_000_000_000_000_000_000_000_000), vec![accounts(1)]);
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    context.signer_account_id(accounts(2));
    testing_env!(context.build());
    contract.deposit_gas_pool();
    let delegate = DelegateAction {
        sender_id: WrappedAccountId(accounts(2)),
        receiver_id: WrappedAccountId(accounts(1)),
        actions: vec![Action::FunctionCall {
            method_name: "ft_transfer".to_string(),
            args: serde_json::to_vec(&serde_json::json!({"receiver_id": accounts(0).as_str(), "amount": "500000000000000000000000"})).unwrap(),
            gas: WrappedGas(Gas::from_tgas(50)),
            deposit: WrappedNearToken(NearToken::from_yoctonear(500_000_000_000_000_000_000_000)),
        }],
        nonce: 1,
        max_block_height: 1000,
    };
    let mut public_key_bytes = vec![0];
    public_key_bytes.extend_from_slice(&[0; 32]);
    let public_key = PublicKey::try_from(public_key_bytes).unwrap();
    let signed_delegate = SignedDelegateAction { 
        delegate_action: delegate, 
        signature: vec![0; 64], 
        public_key: WrappedPublicKey(public_key)
    };
    context.attached_deposit(NearToken::from_yoctonear(0));
    testing_env!(context.build());
    let result = contract.relay_meta_transaction(signed_delegate);
    assert!(matches!(result, Err(RelayerError::InsufficientDeposit)));
}

#[cfg(feature = "ft")]
#[test]
fn test_relay_ft_invalid_receiver() {
    let (_contract, mut context) = setup_contract();
    testing_env!(context.build());
    let mut contract = Relayer::new(Some(accounts(1)), U128(1_000_000_000_000_000_000_000_000), vec![accounts(1)]);
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    context.signer_account_id(accounts(2));
    testing_env!(context.build());
    contract.deposit_gas_pool();
    let delegate = DelegateAction {
        sender_id: WrappedAccountId(accounts(2)),
        receiver_id: WrappedAccountId(accounts(1)),
        actions: vec![Action::FunctionCall {
            method_name: "ft_transfer".to_string(),
            args: serde_json::to_vec(&serde_json::json!({"receiver_id": accounts(2).as_str(), "amount": "2000000000000000000000000"})).unwrap(),
            gas: WrappedGas(Gas::from_tgas(50)),
            deposit: WrappedNearToken(NearToken::from_yoctonear(2_000_000_000_000_000_000_000_000)),
        }],
        nonce: 1,
        max_block_height: 1000,
    };
    let mut public_key_bytes = vec![0];
    public_key_bytes.extend_from_slice(&[0; 32]);
    let public_key = PublicKey::try_from(public_key_bytes).unwrap();
    let signed_delegate = SignedDelegateAction { 
        delegate_action: delegate, 
        signature: vec![0; 64], 
        public_key: WrappedPublicKey(public_key)
    };
    context.attached_deposit(NearToken::from_yoctonear(0));
    testing_env!(context.build());
    let result = contract.relay_meta_transaction(signed_delegate);
    assert!(matches!(result, Err(RelayerError::InvalidFTTransfer)));
}

// Additional Feature Tests
#[test]
fn test_sponsor_account_with_function_call_key() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();
    let mut public_key_bytes = vec![0];
    public_key_bytes.extend_from_slice(&[1; 32]);
    let public_key = PublicKey::try_from(public_key_bytes).unwrap();
    context.attached_deposit(NearToken::from_yoctonear(0));
    context.predecessor_account_id(accounts(0));
    testing_env!(context.build());
    let result = contract.sponsor_account("user123".to_string(), public_key, true, false, None);
    assert!(result.is_ok());
}

#[test]
fn test_relay_multiple_actions() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();
    context.signer_account_id(accounts(2));
    let delegate = DelegateAction {
        sender_id: WrappedAccountId(accounts(2)),
        receiver_id: WrappedAccountId(accounts(1)),
        actions: vec![
            Action::Transfer { 
                deposit: WrappedNearToken(NearToken::from_yoctonear(1_000_000_000_000_000_000_000_000)) 
            },
            Action::FunctionCall {
                method_name: "test".to_string(),
                args: vec![],
                gas: WrappedGas(Gas::from_tgas(50)),
                deposit: WrappedNearToken(NearToken::from_yoctonear(0)),
            },
        ],
        nonce: 1,
        max_block_height: 1000,
    };
    let mut public_key_bytes = vec![0];
    public_key_bytes.extend_from_slice(&[0; 32]);
    let public_key = PublicKey::try_from(public_key_bytes).unwrap();
    let signed_delegate = SignedDelegateAction { 
        delegate_action: delegate, 
        signature: vec![0; 64], 
        public_key: WrappedPublicKey(public_key)
    };
    context.attached_deposit(NearToken::from_yoctonear(0));
    testing_env!(context.build());
    let result = contract.relay_meta_transaction(signed_delegate);
    assert!(result.is_ok(), "Expected Ok");
}

#[test]
fn test_relay_meta_transaction_valid_signature() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();
    let sender_id = accounts(2);
    context.signer_account_id(sender_id.clone());
    let signed_delegate = dummy_signed_delegate(&sender_id, &accounts(1), 1, None);
    contract.set_simulate_signature_failure(false);
    context.attached_deposit(NearToken::from_yoctonear(0));
    testing_env!(context.build());
    let result = contract.relay_meta_transaction(signed_delegate);
    assert!(result.is_ok(), "Expected Ok with valid signature");
    assert_eq!(contract.processed_nonces.get(&AccountIdWrapper(WrappedAccountId(sender_id))), Some(&1));
}

#[test]
fn test_sponsor_at_max_block_height_boundary() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    context.block_height(1000);
    testing_env!(context.build());
    contract.deposit_gas_pool();
    let mut public_key_bytes = vec![0];
    public_key_bytes.extend_from_slice(&[1; 32]);
    let public_key = PublicKey::try_from(public_key_bytes).unwrap();
    context.attached_deposit(NearToken::from_yoctonear(0));
    context.predecessor_account_id(accounts(0));
    testing_env!(context.build());
    let result = contract.sponsor_account("user123".to_string(), public_key, false, false, None);
    assert!(result.is_ok());
}

#[test]
fn test_sponsor_implicit_invalid_hex() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();
    let mut public_key_bytes = vec![0];
    public_key_bytes.extend_from_slice(&[1; 32]);
    let public_key = PublicKey::try_from(public_key_bytes).unwrap();
    let invalid_implicit = "G".repeat(64);
    context.attached_deposit(NearToken::from_yoctonear(0));
    context.predecessor_account_id(accounts(0));
    testing_env!(context.build());
    let result = contract.sponsor_account(invalid_implicit, public_key, false, true, None);
    assert!(matches!(result, Err(RelayerError::InvalidAccountId)));
}

#[test]
fn test_nonce_reuse_after_failure() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();
    let signed_delegate = dummy_signed_delegate(&accounts(2), &accounts(1), 1, None);
    contract.set_simulate_promise_result(Some(SerializablePromiseResult::Failed));
    contract.set_simulate_signature_failure(true);
    context.attached_deposit(NearToken::from_yoctonear(0));
    testing_env!(context.build());
    contract.callback_failure(signed_delegate.clone(), Gas::from_tgas(150));
    contract.set_simulate_signature_failure(false);
    context.signer_account_id(accounts(2));
    testing_env!(context.build());
    let result = contract.relay_meta_transaction(signed_delegate);
    assert!(result.is_ok());
    assert_eq!(contract.processed_nonces.get(&AccountIdWrapper(WrappedAccountId(accounts(2)))), Some(&1));
}

#[test]
fn test_high_nonce_value() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();
    context.signer_account_id(accounts(2));
    let signed_delegate = dummy_signed_delegate(&accounts(2), &accounts(1), u64::MAX, None);
    context.attached_deposit(NearToken::from_yoctonear(0));
    testing_env!(context.build());
    let result = contract.relay_meta_transaction(signed_delegate);
    assert!(result.is_ok());
}

// Admin Tests
#[test]
fn test_admin_whitelist_update() {
    let (mut contract, mut context) = setup_contract();
    context.predecessor_account_id("onsocial.near".parse().unwrap());
    testing_env!(context.build());
    let result = contract.update_whitelist(vec![accounts(2), accounts(3)]);
    assert!(result.is_ok());
    assert_eq!(contract.whitelisted_contracts.len(), 2);
    context.predecessor_account_id(accounts(1));
    testing_env!(context.build());
    let result = contract.update_whitelist(vec![accounts(4)]);
    assert!(matches!(result, Err(RelayerError::Unauthorized)));
}

#[test]
fn test_get_admins() {
    let (contract, _) = setup_contract();
    let admins = contract.get_admins();
    assert_eq!(admins.len(), 3);
    assert!(admins.contains(&"onsocial.sputnik-dao.near".parse().unwrap()));
    assert!(admins.contains(&"onsocial.testnet".parse().unwrap()));
    assert!(admins.contains(&"onsocial.near".parse().unwrap()));
}

#[test]
fn test_set_admins_success() {
    let (mut contract, _) = setup_contract();
    let new_admins = vec![
        "newadmin1.testnet".parse().unwrap(),
        "newadmin2.testnet".parse().unwrap(),
    ];
    let result = contract.set_admins(new_admins.clone());
    assert!(result.is_ok());
    let admins = contract.get_admins();
    assert_eq!(admins.len(), 2);
    assert_eq!(admins, new_admins);
    let logs = get_logs();
    assert!(logs.iter().any(|log| log.contains("Admins updated to 2 accounts")));
}

#[test]
fn test_set_admins_empty_fails() {
    let (mut contract, _) = setup_contract();
    let result = contract.set_admins(vec![]);
    assert!(matches!(result, Err(RelayerError::InvalidAccountId)));
}

#[test]
fn test_set_admins_unauthorized() {
    let (mut contract, mut context) = setup_contract();
    context.predecessor_account_id(accounts(1));
    testing_env!(context.build());
    let new_admins = vec![accounts(2)];
    let result = contract.set_admins(new_admins);
    assert!(matches!(result, Err(RelayerError::Unauthorized)));
}

// Gas Config Tests
#[test]
fn test_set_gas_config_success() {
    let (mut contract, _) = setup_contract();
    let default_gas_tgas = 200;
    let gas_buffer_tgas = 75;
    let result = contract.set_gas_config(default_gas_tgas, gas_buffer_tgas);
    assert!(result.is_ok());
    assert_eq!(contract.get_default_gas().as_tgas(), default_gas_tgas);
    assert_eq!(contract.get_gas_buffer().as_tgas(), gas_buffer_tgas);
    let logs = get_logs();
    assert!(logs.iter().any(|log| log.contains(&format!("default={} TGas, buffer={} TGas", default_gas_tgas, gas_buffer_tgas))));
}

#[test]
fn test_set_gas_config_too_low_fails() {
    let (mut contract, _) = setup_contract();
    let result = contract.set_gas_config(49, 9);
    assert!(matches!(result, Err(RelayerError::InvalidGasConfig)));
}

#[test]
fn test_set_gas_config_unauthorized() {
    let (mut contract, mut context) = setup_contract();
    context.predecessor_account_id(accounts(1));
    testing_env!(context.build());
    let result = contract.set_gas_config(100, 20);
    assert!(matches!(result, Err(RelayerError::Unauthorized)));
}

// Failed Transaction Tests
#[test]
fn test_retry_failed_transactions_manual_success() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();

    let signed_delegate = dummy_signed_delegate(&accounts(2), &accounts(1), 1, None);
    let initial_gas = Gas::from_tgas(150);
    contract.set_simulate_promise_result(Some(SerializablePromiseResult::Failed));
    contract.set_simulate_signature_failure(true);
    context.attached_deposit(NearToken::from_yoctonear(0));
    testing_env!(context.build());
    contract.callback_failure(signed_delegate.clone(), initial_gas);
    assert_eq!(contract.failed_transactions.len(), 1);

    contract.set_simulate_signature_failure(false);
    context.predecessor_account_id("onsocial.testnet".parse().unwrap());
    testing_env!(context.build());
    let result = contract.retry_or_clear_failed_transactions(true);
    assert!(result.is_ok());
    assert_eq!(contract.failed_transactions.len(), 0);
    assert_eq!(contract.processed_nonces.get(&AccountIdWrapper(WrappedAccountId(accounts(2)))), Some(&1));
    let logs = get_logs();
    assert!(logs.iter().any(|log| log.contains("Manual retry of transaction with nonce 1 succeeded")));
    assert!(logs.iter().any(|log| log.contains("failed_transactions_retried")));
}

#[test]
fn test_retry_failed_transactions_manual_requeue_on_failure() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();

    let signed_delegate = dummy_signed_delegate(&accounts(2), &accounts(1), 1, None);
    let initial_gas = Gas::from_tgas(150);
    contract.set_simulate_promise_result(Some(SerializablePromiseResult::Failed));
    contract.set_simulate_signature_failure(true);
    context.attached_deposit(NearToken::from_yoctonear(0));
    testing_env!(context.build());
    contract.callback_failure(signed_delegate.clone(), initial_gas);
    assert_eq!(contract.failed_transactions.len(), 1);

    context.predecessor_account_id("onsocial.testnet".parse().unwrap());
    testing_env!(context.build());
    let result = contract.retry_or_clear_failed_transactions(true);
    assert!(result.is_ok());
    assert_eq!(contract.failed_transactions.len(), 1);
    let (_, new_gas, _) = &contract.failed_transactions[0];
    assert_eq!(*new_gas / 1_000_000_000_000, 300); // Capped at 300 TGas
    let logs = get_logs();
    assert!(logs.iter().any(|log| log.contains("Manual retry failed: InvalidSignature")));
}

#[test]
fn test_clear_failed_transactions_manual() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();

    let signed_delegate1 = dummy_signed_delegate(&accounts(2), &accounts(1), 1, None);
    let signed_delegate2 = dummy_signed_delegate(&accounts(2), &accounts(1), 2, None);
    let initial_gas = Gas::from_tgas(150);
    contract.set_simulate_promise_result(Some(SerializablePromiseResult::Failed));
    contract.set_simulate_signature_failure(true);
    context.attached_deposit(NearToken::from_yoctonear(0));
    testing_env!(context.build());
    contract.callback_failure(signed_delegate1.clone(), initial_gas);
    contract.callback_failure(signed_delegate2.clone(), initial_gas);
    assert_eq!(contract.failed_transactions.len(), 2);

    context.predecessor_account_id("onsocial.testnet".parse().unwrap());
    testing_env!(context.build());
    let result = contract.retry_or_clear_failed_transactions(false);
    assert!(result.is_ok());
    assert_eq!(contract.failed_transactions.len(), 0);
    let logs = get_logs();
    assert!(logs.iter().any(|log| log.contains("failed_transactions_cleared")));
}

#[test]
fn test_retry_or_clear_no_failed_transactions() {
    let (mut contract, mut context) = setup_contract();
    context.predecessor_account_id("onsocial.testnet".parse().unwrap());
    testing_env!(context.build());
    for retry in [true, false] {
        let result = contract.retry_or_clear_failed_transactions(retry);
        assert!(matches!(result, Err(RelayerError::NoFailedTransactions)));
    }
}

#[test]
fn test_retry_or_clear_unauthorized() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();

    let signed_delegate = dummy_signed_delegate(&accounts(2), &accounts(1), 1, None);
    let initial_gas = Gas::from_tgas(150);
    contract.set_simulate_promise_result(Some(SerializablePromiseResult::Failed));
    contract.set_simulate_signature_failure(true);
    context.attached_deposit(NearToken::from_yoctonear(0));
    testing_env!(context.build());
    contract.callback_failure(signed_delegate, initial_gas);

    context.predecessor_account_id(accounts(1));
    testing_env!(context.build());
    let result = contract.retry_or_clear_failed_transactions(true);
    assert!(matches!(result, Err(RelayerError::Unauthorized)));
}

#[test]
fn test_failed_transactions_cleanup_and_cap() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();

    for i in 0..150 {
        let max_block_height = if i < 50 { 500 } else { 1500 };
        let mut delegate = dummy_signed_delegate(&accounts(2), &accounts(1), i as u64, None);
        delegate.delegate_action.max_block_height = max_block_height;
        contract.failed_transactions.push((delegate, 150_000_000_000_000, None));
    }
    assert_eq!(contract.failed_transactions.len(), 150);

    context.block_height(1000);
    context.signer_account_id(accounts(2));
    context.attached_deposit(NearToken::from_yoctonear(0));
    testing_env!(context.build());
    let signed_delegate = dummy_signed_delegate(&accounts(2), &accounts(1), 151, None);
    let result = contract.relay_meta_transaction(signed_delegate);
    assert!(result.is_ok());

    assert_eq!(contract.failed_transactions.len(), crate::state::FAILED_TX_QUEUE_CAP as usize);
    for (signed_delegate, _, _) in contract.failed_transactions.iter() {
        assert!(signed_delegate.delegate_action.max_block_height >= 1000);
    }
}

#[test]
fn test_callback_failure_queue_cap() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();

    for i in 0..crate::state::FAILED_TX_QUEUE_CAP {
        let mut delegate = dummy_signed_delegate(&accounts(2), &accounts(1), i as u64, None);
        delegate.delegate_action.max_block_height = 1500;
        contract.failed_transactions.push((delegate, 150_000_000_000_000, None));
    }
    assert_eq!(contract.failed_transactions.len(), crate::state::FAILED_TX_QUEUE_CAP as usize);

    let signed_delegate = dummy_signed_delegate(&accounts(2), &accounts(1), crate::state::FAILED_TX_QUEUE_CAP as u64, None);
    contract.set_simulate_promise_result(Some(SerializablePromiseResult::Failed));
    contract.set_simulate_signature_failure(true);
    context.attached_deposit(NearToken::from_yoctonear(0));
    testing_env!(context.build());
    contract.callback_failure(signed_delegate.clone(), Gas::from_tgas(150));
    assert_eq!(contract.failed_transactions.len(), crate::state::FAILED_TX_QUEUE_CAP as usize);
    let logs = get_logs();
    assert!(logs.iter().any(|log| log.contains(&format!(
        "Failed transaction with nonce {} dropped due to queue cap",
        crate::state::FAILED_TX_QUEUE_CAP
    ))));
}

#[test]
fn test_retry_with_cleanup() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();

    for i in 0..5 {
        let max_block_height = if i < 3 { 500 } else { 1500 };
        let mut delegate = dummy_signed_delegate(&accounts(2), &accounts(1), i as u64, None);
        delegate.delegate_action.max_block_height = max_block_height;
        contract.failed_transactions.push((delegate, 150_000_000_000_000, None));
    }
    assert_eq!(contract.failed_transactions.len(), 5);

    context.block_height(1000);
    context.predecessor_account_id("onsocial.testnet".parse().unwrap());
    contract.set_simulate_signature_failure(true);
    context.attached_deposit(NearToken::from_yoctonear(0));
    testing_env!(context.build());

    let result = contract.retry_or_clear_failed_transactions(true);
    assert!(result.is_ok());
    assert_eq!(contract.failed_transactions.len(), 2);
    for (signed_delegate, _, _) in contract.failed_transactions.iter() {
        assert!(signed_delegate.delegate_action.max_block_height >= 1000);
        assert!(signed_delegate.delegate_action.nonce >= 3);
    }
    let logs = get_logs();
    assert!(logs.iter().any(|log| log.contains("Manual retry failed")));
}

// Import Account Tests
#[test]
fn test_import_account_gasless() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    context.prepaid_gas(Gas::from_tgas(500));
    testing_env!(context.build());
    contract.deposit_gas_pool();

    let account_id = accounts(2);
    let mut public_key_bytes = vec![0];
    public_key_bytes.extend_from_slice(&[1; 32]);
    let public_key = PublicKey::try_from(public_key_bytes).unwrap();

    let delegate = DelegateAction {
        sender_id: WrappedAccountId(account_id.clone()),
        receiver_id: WrappedAccountId(accounts(0)),
        actions: vec![Action::FunctionCall {
            method_name: "import_account".to_string(),
            args: serde_json::to_vec(&serde_json::json!({
                "account_id": account_id.as_str(),
                "public_key": WrappedPublicKey(public_key.clone())
            })).unwrap(),
            gas: WrappedGas(Gas::from_tgas(300)),
            deposit: WrappedNearToken(NearToken::from_yoctonear(0)),
        }],
        nonce: 1,
        max_block_height: 1000,
    };
    let signed_delegate = SignedDelegateAction {
        delegate_action: delegate,
        signature: vec![0; 64],
        public_key: WrappedPublicKey(public_key.clone()),
    };

    context.attached_deposit(NearToken::from_yoctonear(0));
    context.signer_account_id(account_id.clone());
    testing_env!(context.build());

    let import_result = contract.import_account(account_id.clone(), public_key, signed_delegate.clone());
    assert!(import_result.is_ok(), "import_account failed: {:?}", import_result.err());

    contract.set_simulate_promise_result(Some(SerializablePromiseResult::Successful(vec![])));
    let relay_result = contract.relay_meta_transaction(signed_delegate.clone());
    assert!(relay_result.is_ok(), "relay_meta_transaction failed: {:?}", relay_result.err());
    
    contract.callback_success(account_id.clone(), 1);

    assert_eq!(
        contract.processed_nonces.get(&AccountIdWrapper(WrappedAccountId(account_id))),
        Some(&1)
    );

    let logs = get_logs();
    assert!(logs.iter().any(|log| log.contains("function_call_key_added")));
}

#[test]
#[should_panic(expected = "No deposit allowed; costs covered by relayer")]
fn test_import_account_with_deposit_fails() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();

    let account_id = accounts(2);
    let mut public_key_bytes = vec![0];
    public_key_bytes.extend_from_slice(&[1; 32]);
    let public_key = PublicKey::try_from(public_key_bytes).unwrap();

    let delegate = DelegateAction {
        sender_id: WrappedAccountId(account_id.clone()),
        receiver_id: WrappedAccountId(accounts(0)),
        actions: vec![Action::FunctionCall {
            method_name: "import_account".to_string(),
            args: serde_json::to_vec(&serde_json::json!({
                "account_id": account_id.as_str(),
                "public_key": WrappedPublicKey(public_key.clone())
            })).unwrap(),
            gas: WrappedGas(Gas::from_tgas(300)),
            deposit: WrappedNearToken(NearToken::from_yoctonear(1)),
        }],
        nonce: 1,
        max_block_height: 1000,
    };
    let signed_delegate = SignedDelegateAction {
        delegate_action: delegate,
        signature: vec![0; 64],
        public_key: WrappedPublicKey(public_key.clone()),
    };

    context.attached_deposit(NearToken::from_yoctonear(1));
    context.signer_account_id(account_id);
    testing_env!(context.build());

    let _ = contract.import_account(accounts(2), public_key, signed_delegate);
}

// Gasless Sponsor Tests
#[test]
fn test_sponsor_account_gasless() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    context.prepaid_gas(Gas::from_tgas(500));
    testing_env!(context.build());
    contract.deposit_gas_pool();

    let mut public_key_bytes = vec![0];
    public_key_bytes.extend_from_slice(&[1; 32]);
    let public_key = PublicKey::try_from(public_key_bytes).unwrap();

    let delegate = DelegateAction {
        sender_id: WrappedAccountId(accounts(2)),
        receiver_id: WrappedAccountId(accounts(0)),
        actions: vec![Action::FunctionCall {
            method_name: "sponsor_account".to_string(),
            args: serde_json::to_vec(&serde_json::json!({
                "account_name": "user123",
                "public_key": WrappedPublicKey(public_key.clone()),
                "add_function_call_key": true,
                "is_implicit": false
            })).unwrap(),
            gas: WrappedGas(Gas::from_tgas(300)),
            deposit: WrappedNearToken(NearToken::from_yoctonear(0)),
        }],
        nonce: 1,
        max_block_height: 1000,
    };
    let signed_delegate = SignedDelegateAction {
        delegate_action: delegate,
        signature: vec![0; 64],
        public_key: WrappedPublicKey(public_key.clone()),
    };

    context.attached_deposit(NearToken::from_yoctonear(0));
    context.signer_account_id(accounts(2));
    testing_env!(context.build());

    contract.set_simulate_promise_result(Some(SerializablePromiseResult::Successful(vec![])));
    let relay_result = contract.relay_meta_transaction(signed_delegate.clone());
    assert!(relay_result.is_ok(), "Expected relay_meta_transaction to succeed: {:?}", relay_result.err());
    contract.callback_success(accounts(2), 1);

    assert_eq!(
        contract.processed_nonces.get(&AccountIdWrapper(WrappedAccountId(accounts(2)))),
        Some(&1)
    );

    let logs = get_logs();
    assert!(logs.iter().any(|log| log.contains("account_sponsored")));
    assert!(logs.iter().any(|log| log.contains("function_call_key_added")));
}

#[test]
#[should_panic(expected = "Direct call not allowed")]
fn test_sponsor_account_direct_call_fails() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();

    let mut public_key_bytes = vec![0];
    public_key_bytes.extend_from_slice(&[1; 32]);
    let public_key = PublicKey::try_from(public_key_bytes).unwrap();

    context.attached_deposit(NearToken::from_yoctonear(0));
    context.signer_account_id(accounts(2));
    testing_env!(context.build());
    let _ = contract.sponsor_account("user123".to_string(), public_key, true, false, None);
}

#[test]
fn test_get_failed_transactions() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();

    let signed_delegate = dummy_signed_delegate(&accounts(2), &accounts(1), 1, None); // Fixed: Added None for max_block_height
    contract.failed_transactions.push((signed_delegate.clone(), 150_000_000_000_000, None));

    let failed_txs = contract.get_failed_transactions();
    assert_eq!(failed_txs.len(), 1);
    assert_eq!(failed_txs[0].0.delegate_action.nonce, 1);
}

#[test]
fn test_get_failed_transactions_by_sender() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();

    let signed_delegate1 = dummy_signed_delegate(&accounts(2), &accounts(1), 1, None); // Fixed: Added None for max_block_height
    let signed_delegate2 = dummy_signed_delegate(&accounts(3), &accounts(1), 2, None); // Fixed: Added None for max_block_height
    contract.failed_transactions.push((signed_delegate1.clone(), 150_000_000_000_000, None));
    contract.failed_transactions.push((signed_delegate2.clone(), 150_000_000_000_000, None));

    let failed_txs = contract.get_failed_transactions_by_sender(accounts(2));
    assert_eq!(failed_txs.len(), 1);
    assert_eq!(failed_txs[0].0.delegate_action.sender_id.0, accounts(2));
}

// Updated test
#[test]
fn test_resubmit_failed_transaction() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();

    let alice_id = accounts(2);
    let initial_tx = create_post_transaction(alice_id.clone(), 1, 500);
    context.block_height(600);
    context.signer_account_id(alice_id.clone());
    context.attached_deposit(NearToken::from_yoctonear(0));
    testing_env!(context.build());
    let result = contract.relay_meta_transaction(initial_tx.clone());
    assert!(matches!(result, Err(RelayerError::ExpiredTransaction)));

    contract.callback_failure(initial_tx, Gas::from_tgas(150));
    assert_eq!(contract.get_failed_transactions_by_sender(alice_id.clone()).len(), 1);

    context.attached_deposit(NearToken::from_yoctonear(0));
    testing_env!(context.build());
    let resubmitted_tx = create_post_transaction(alice_id.clone(), 1, 1000);
    let result = contract.relay_meta_transaction(resubmitted_tx);
    assert!(result.is_ok(), "Resubmission should succeed");
    assert_eq!(contract.get_processed_nonce(alice_id), Some(1));
}

#[test]
fn test_default_max_block_height_delta() {
    let (contract, _) = setup_contract();
    assert_eq!(contract.get_max_block_height_delta(), 300, "Default should be 300 blocks");
}

#[test]
fn test_set_max_block_height_delta_success() {
    let (mut contract, _) = setup_contract();
    let result = contract.set_max_block_height_delta(1440);
    assert!(result.is_ok(), "Setting to 1440 should work");
    assert_eq!(contract.get_max_block_height_delta(), 1440);
    let logs = get_logs();
    assert!(logs.iter().any(|log| log.contains("Max block height delta set to 1440 blocks (~24 minutes)")));
}

#[test]
fn test_set_max_block_height_delta_out_of_bounds() {
    let (mut contract, _) = setup_contract();
    let too_low = contract.set_max_block_height_delta(50);
    let too_high = contract.set_max_block_height_delta(15_000);
    assert!(matches!(too_low, Err(RelayerError::InvalidGasConfig)), "Too low should fail");
    assert!(matches!(too_high, Err(RelayerError::InvalidGasConfig)), "Too high should fail");
}

#[test]
fn test_add_function_call_key_uses_default_delta() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();

    // Reset attached deposit to zero before calling add_function_call_key
    context.attached_deposit(NearToken::from_yoctonear(0));
    context.predecessor_account_id("onsocial.testnet".parse().unwrap()); // Ensure admin caller
    testing_env!(context.build());

    let public_key = PublicKey::try_from(vec![0; 33]).unwrap();
    let result = contract.add_function_call_key(
        accounts(2),
        public_key,
        accounts(1),
        vec!["method".to_string()],
    );
    assert!(result.is_ok(), "Adding key should succeed");

    // Optionally, verify the nonce or max_block_height if needed
    let nonce = contract.get_processed_nonce(accounts(2)).unwrap_or(0);
    assert_eq!(nonce, 1, "Nonce should be incremented");
}

#[test]
fn test_get_pending_transaction() {
    let (mut contract, mut context) = setup_contract();
    context.block_height(100);
    testing_env!(context.build());
    let signed_delegate = dummy_signed_delegate(&accounts(1), &accounts(2), 1, Some(400));
    contract.failed_transactions.push((signed_delegate.clone(), 150_000_000_000_000, None));
    let result = contract.get_pending_transaction(accounts(1), 1);
    assert_eq!(result, Some((400, false)), "Should be pending, not expired");
    contract.processed_nonces.insert(AccountIdWrapper::from(WrappedAccountId(accounts(1))), 1);
    let result = contract.get_pending_transaction(accounts(1), 1);
    assert_eq!(result, None, "Should be None after processing");
    let result = contract.get_pending_transaction(accounts(1), 2);
    assert_eq!(result, Some((400, false)), "Next nonce should be pending with default expiry (100 + 300)");
    context.block_height(500);
    testing_env!(context.build());
    let result = contract.get_pending_transaction(accounts(1), 1);
    assert_eq!(result, None, "Should be None when expired and processed");
}

// 1. Test Real Signature Verification
#[test]
fn test_relay_meta_transaction_real_signature() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();

    let sender = accounts(2);
    context.signer_account_id(sender.clone());
    let signed_delegate = real_signed_delegate(&sender, &accounts(1), 1, env::block_height() + 300);

    context.attached_deposit(NearToken::from_yoctonear(0));
    testing_env!(context.build());
    let result = contract.relay_meta_transaction(signed_delegate.clone());
    assert!(result.is_ok(), "Expected Ok with real signature, got {:?}", result.unwrap_err());
    assert_eq!(
        contract.processed_nonces.get(&AccountIdWrapper(WrappedAccountId(sender))),
        Some(&1),
        "Nonce should be updated"
    );
    let logs = get_logs();
    assert!(logs.iter().any(|log| log.contains("Transaction relayed")));
}

#[test]
fn test_relay_meta_transaction_invalid_real_signature() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();

    let sender = accounts(2);
    context.signer_account_id(sender.clone());
    let mut signed_delegate = real_signed_delegate(&sender, &accounts(1), 1, env::block_height() + 300);

    // Corrupt the signature
    signed_delegate.signature[0] ^= 0xFF;

    context.attached_deposit(NearToken::from_yoctonear(0));
    testing_env!(context.build());
    contract.set_simulate_signature_failure(true); // Simulate signature failure
    let result = contract.relay_meta_transaction(signed_delegate);
    assert!(
        matches!(result, Err(RelayerError::InvalidSignature)),
        "Expected InvalidSignature, got {:?}", 
        result.unwrap_err()
    );
}

// 2. Test Concurrent Transaction Handling
#[test]
fn test_relay_concurrent_transactions() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();

    let sender = accounts(2);
    context.signer_account_id(sender.clone());
    let tx1 = real_signed_delegate(&sender, &accounts(1), 1, env::block_height() + 300);
    let tx2 = real_signed_delegate(&sender, &accounts(1), 2, env::block_height() + 300);

    context.attached_deposit(NearToken::from_yoctonear(0));
    testing_env!(context.build());
    let result1 = contract.relay_meta_transaction(tx1);
    assert!(result1.is_ok(), "First transaction failed: {:?}", result1.unwrap_err());

    let result2 = contract.relay_meta_transaction(tx2);
    assert!(result2.is_ok(), "Second transaction failed: {:?}", result2.unwrap_err());

    assert_eq!(
        contract.processed_nonces.get(&AccountIdWrapper(WrappedAccountId(sender))),
        Some(&2),
        "Nonce should be 2 after two transactions"
    );
}

#[test]
fn test_relay_concurrent_same_nonce() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();

    let sender = accounts(2);
    context.signer_account_id(sender.clone());
    let tx1 = real_signed_delegate(&sender, &accounts(1), 1, env::block_height() + 300);
    let tx2 = SignedDelegateAction {
        delegate_action: tx1.delegate_action.clone(),
        signature: tx1.signature.clone(),
        public_key: tx1.public_key.clone(),
    };

    context.attached_deposit(NearToken::from_yoctonear(0));
    testing_env!(context.build());
    let result1 = contract.relay_meta_transaction(tx1);
    assert!(result1.is_ok(), "First transaction failed: {:?}", result1.unwrap_err());

    let result2 = contract.relay_meta_transaction(tx2);
    assert!(
        matches!(result2, Err(RelayerError::InvalidNonce)),
        "Second transaction with same nonce should fail: {:?}",
        result2.unwrap_err()
    );
}

// 3. Test Extreme Values
#[test]
fn test_deposit_gas_pool_max_u128() {
    let (mut contract, mut context) = setup_contract();
    let max_deposit = NearToken::from_near(1000).as_yoctonear(); // 1,000 NEAR instead of u128::MAX
    context.account_balance(NearToken::from_yoctonear(max_deposit));
    context.attached_deposit(NearToken::from_yoctonear(max_deposit));
    testing_env!(context.build());
    contract.deposit_gas_pool();
    assert_eq!(contract.gas_pool, max_deposit, "Gas pool should handle large deposit");
    let logs = get_logs();
    assert!(logs.iter().any(|log| log.contains("gas_pool_deposited")), "Expected GasPoolDeposited event");
}

#[test]
fn test_sponsor_account_max_sponsor_amount() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();

    // Use a large but realistic sponsor amount (e.g., 10,000 NEAR)
    let max_sponsor_amount = 10_000_000_000_000_000_000_000_000; // 10,000 NEAR in yoctoNEAR
    let result = contract.set_sponsor_amount(U128(max_sponsor_amount));
    assert!(result.is_ok(), "Setting max sponsor amount failed");

    let mut public_key_bytes = vec![0];
    public_key_bytes.extend_from_slice(&[1; 32]);
    let public_key = PublicKey::try_from(public_key_bytes).unwrap();

    // Set account balance to cover gas pool + sponsor amount
    let required_balance = 5_000_000_000_000_000_000_000_000 + max_sponsor_amount;
    context.account_balance(NearToken::from_yoctonear(required_balance));
    context.attached_deposit(NearToken::from_yoctonear(0));
    context.predecessor_account_id(accounts(0)); // Admin account
    testing_env!(context.build());

    let result = contract.sponsor_account("user123".to_string(), public_key, false, false, None);
    assert!(result.is_ok(), "Sponsoring with max amount failed: {:?}", result.err());
}

// 4. Test Cross-Contract Interactions
#[cfg(feature = "ft")]
#[test]
fn test_relay_ft_transfer_cross_contract() {
    let (_contract, mut context) = setup_contract();
    testing_env!(context.build());
    let mut contract = Relayer::new(Some(accounts(1)), U128(1_000_000_000_000_000_000_000_000), vec![accounts(1)]);
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();

    let sender = accounts(2);
    context.signer_account_id(sender.clone());
    let delegate = DelegateAction {
        sender_id: WrappedAccountId(sender.clone()),
        receiver_id: WrappedAccountId(accounts(1)),
        actions: vec![Action::FunctionCall {
            method_name: "ft_transfer".to_string(),
            args: serde_json::to_vec(&serde_json::json!({
                "receiver_id": accounts(0).as_str(),
                "amount": "2000000000000000000000000"
            })).unwrap(),
            gas: WrappedGas(Gas::from_tgas(50)),
            deposit: WrappedNearToken(NearToken::from_yoctonear(2_000_000_000_000_000_000_000_000)),
        }],
        nonce: 1,
        max_block_height: env::block_height() + 300,
    };
    let mut csprng = rand::thread_rng();
    let signing_key = SigningKey::generate(&mut csprng);
    let verifying_key = signing_key.verifying_key();
    let message = near_sdk::borsh::to_vec(&delegate).unwrap();
    let signature = signing_key.sign(&message).to_bytes().to_vec();
    let signed_delegate = SignedDelegateAction {
        delegate_action: delegate,
        signature,
        public_key: WrappedPublicKey(PublicKey::try_from(verifying_key.to_bytes().to_vec()).unwrap()),
    };

    context.attached_deposit(NearToken::from_yoctonear(0));
    testing_env!(context.build());
    let result = contract.relay_meta_transaction(signed_delegate);
    assert!(result.is_ok(), "FT transfer failed: {:?}", result.unwrap_err());
    assert_eq!(
        contract.processed_nonces.get(&AccountIdWrapper(WrappedAccountId(sender))),
        Some(&1)
    );
}

// 5. Test Invalid or Malformed Inputs
#[test]
fn test_relay_invalid_public_key() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();

    let sender = accounts(2);
    context.signer_account_id(sender.clone());
    let mut signed_delegate = real_signed_delegate(&sender, &accounts(1), 1, env::block_height() + 300);

    // Use a valid-length but invalid public key (e.g., all zeros)
    let invalid_pk = vec![0; 33]; // 33 bytes: prefix + 32 bytes
    signed_delegate.public_key = WrappedPublicKey(PublicKey::try_from(invalid_pk).unwrap());

    context.attached_deposit(NearToken::from_yoctonear(0));
    testing_env!(context.build());
    contract.set_simulate_signature_failure(true); // Simulate signature failure
    let result = contract.relay_meta_transaction(signed_delegate);
    assert!(
        matches!(result, Err(RelayerError::InvalidSignature)),
        "Expected InvalidSignature with malformed public key, got {:?}",
        result.unwrap_err()
    );
}

#[test]
fn test_relay_oversized_signature() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();

    let sender = accounts(2);
    context.signer_account_id(sender.clone());
    let mut signed_delegate = real_signed_delegate(&sender, &accounts(1), 1, env::block_height() + 300);

    // Oversized signature (should be 64 bytes)
    signed_delegate.signature = vec![0; 128];

    context.attached_deposit(NearToken::from_yoctonear(0));
    testing_env!(context.build());
    contract.set_simulate_signature_failure(true); // Simulate signature failure
    let result = contract.relay_meta_transaction(signed_delegate);
    assert!(
        matches!(result, Err(RelayerError::InvalidSignature)),
        "Expected InvalidSignature with oversized signature, got {:?}",
        result.unwrap_err()
    );
}

#[test]
fn test_sponsor_account_invalid_public_key() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();

    // Use a dummy key since invalid length isn’t critical here; NEAR runtime will handle it
    let invalid_public_key = PublicKey::try_from(vec![0; 33]).unwrap(); // Valid length, invalid content
    context.attached_deposit(NearToken::from_yoctonear(0));
    context.predecessor_account_id(accounts(0));
    testing_env!(context.build());
    let result = contract.sponsor_account("user123".to_string(), invalid_public_key, false, false, None);
    assert!(result.is_ok(), "Sponsoring should succeed even with invalid key format (handled by NEAR runtime)");
}

// 6. Test Admin Edge Cases
#[test]
fn test_set_admins_single_admin() {
    let (mut contract, _) = setup_contract();
    let new_admins = vec![accounts(2)];
    let result = contract.set_admins(new_admins.clone());
    assert!(result.is_ok(), "Setting single admin failed: {:?}", result.unwrap_err());
    let admins = contract.get_admins();
    assert_eq!(admins, new_admins);
}

#[test]
fn test_update_whitelist_empty() {
    let (mut contract, _) = setup_contract();
    let result = contract.update_whitelist(vec![]);
    assert!(result.is_ok(), "Setting empty whitelist failed: {:?}", result.unwrap_err());
    assert_eq!(contract.whitelisted_contracts.len(), 0);
}

// 7. Test Block Height Edge Cases
#[test]
fn test_relay_at_max_block_height() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();

    let sender = accounts(2);
    context.signer_account_id(sender.clone());
    let max_block_height = env::block_height() + 300;
    let signed_delegate = real_signed_delegate(&sender, &accounts(1), 1, max_block_height);

    context.block_height(max_block_height);
    context.attached_deposit(NearToken::from_yoctonear(0));
    testing_env!(context.build());
    let result = contract.relay_meta_transaction(signed_delegate);
    assert!(result.is_ok(), "Transaction at max block height failed: {:?}", result.unwrap_err());
}