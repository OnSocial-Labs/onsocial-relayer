#[cfg(test)]
use near_sdk::test_utils::{VMContextBuilder, accounts, get_logs};
#[cfg(test)]
use near_sdk::testing_env;
use near_sdk::NearToken;
use crate::types::{SignedDelegateAction, DelegateAction, Action, SerializablePromiseResult, WrappedAccountId, WrappedNearToken, WrappedGas, WrappedPublicKey};
use crate::state::{Relayer, AccountIdWrapper};
use crate::errors::RelayerError;
use near_sdk::Gas;
use near_sdk::json_types::U128;
use near_sdk::PublicKey;
use serde_json;

pub fn setup_contract() -> (Relayer, VMContextBuilder) {
    let mut context = VMContextBuilder::new();
    context.current_account_id(accounts(0));
    context.predecessor_account_id("onsocial.testnet".parse().unwrap());
    context.account_balance(NearToken::from_yoctonear(10_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    let contract = Relayer::new(None, U128(0), vec![accounts(1)]);
    (contract, context)
}

pub fn dummy_signed_delegate(sender: &near_sdk::AccountId, receiver: &near_sdk::AccountId, nonce: u64) -> SignedDelegateAction {
    let delegate = DelegateAction {
        sender_id: WrappedAccountId(sender.clone()),
        receiver_id: WrappedAccountId(receiver.clone()),
        actions: vec![Action::Transfer { 
            deposit: WrappedNearToken(NearToken::from_yoctonear(1_000_000_000_000_000_000_000_000)) 
        }],
        nonce,
        max_block_height: 1000,
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
fn test_relay_meta_transaction_usdc_testnet() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();
    context.signer_account_id(accounts(2));
    let usdc_testnet: near_sdk::AccountId = "3e2210e1184b45b64c8a434c0a7e7b23cc04ea7eb7a6c3c32520d03d4afcb8af".parse().unwrap();
    let signed_delegate = dummy_signed_delegate(&accounts(2), &usdc_testnet, 1);
    testing_env!(context.build());
    let result = contract.relay_meta_transaction(signed_delegate);
    assert!(result.is_ok(), "Expected Ok for USDC testnet, got {:?}", result);
}

#[test]
fn test_relay_meta_transaction_usdc_mainnet() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();
    context.signer_account_id(accounts(2));
    let usdc_mainnet: near_sdk::AccountId = "17208628f84f5d6ad33f0da3bbbeb27ffcb398eac501a31bd6ad2011e36133a1".parse().unwrap();
    let signed_delegate = dummy_signed_delegate(&accounts(2), &usdc_mainnet, 1);
    testing_env!(context.build());
    let result = contract.relay_meta_transaction(signed_delegate);
    assert!(result.is_ok(), "Expected Ok for USDC mainnet, got {:?}", result);
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

#[test]
fn test_sponsor_named_account() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();
    let mut public_key_bytes = vec![0];
    public_key_bytes.extend_from_slice(&[1; 32]);
    let public_key = PublicKey::try_from(public_key_bytes).unwrap();
    let result = contract.sponsor_account("user123".to_string(), public_key, false, false);
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
    let result = contract.sponsor_account(implicit_id, public_key, false, true);
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
    let result = contract.sponsor_account("invalid".to_string(), public_key, false, true);
    assert!(matches!(result, Err(RelayerError::InvalidAccountId)));
}

#[test]
fn test_sponsor_account_insufficient_balance() {
    let (mut contract, mut context) = setup_contract();
    context.account_balance(NearToken::from_yoctonear(1_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    let mut public_key_bytes = vec![0];
    public_key_bytes.extend_from_slice(&[1; 32]);
    let public_key = PublicKey::try_from(public_key_bytes).unwrap();
    let result = contract.sponsor_account("user123".to_string(), public_key, false, false);
    assert!(matches!(result, Err(RelayerError::InsufficientBalance)));
}

#[test]
fn test_sponsor_account_already_exists() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();
    let mut public_key_bytes = vec![0];
    public_key_bytes.extend_from_slice(&[1; 32]);
    let public_key = PublicKey::try_from(public_key_bytes).unwrap();
    let account_id = "user123.testnet".parse().unwrap();
    contract.processed_nonces.insert(AccountIdWrapper(WrappedAccountId(account_id)), 1);
    context.current_account_id("alice.testnet".parse().unwrap());
    testing_env!(context.build());
    let result = contract.sponsor_account("user123".to_string(), public_key, false, false);
    assert!(matches!(result, Err(RelayerError::AccountExists)));
}

#[test]
fn test_relay_meta_transaction_no_ft() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();
    context.signer_account_id(accounts(2));
    let signed_delegate = dummy_signed_delegate(&accounts(2), &accounts(1), 1);
    let result = contract.relay_meta_transaction(signed_delegate);
    assert!(result.is_ok(), "Expected Ok, got {:?}", result);
}

#[test]
fn test_relay_meta_transaction_insufficient_gas_pool() {
    let (mut contract, mut context) = setup_contract();
    context.account_balance(NearToken::from_yoctonear(500_000_000_000_000_000_000_000));
    context.signer_account_id(accounts(2));
    let signed_delegate = dummy_signed_delegate(&accounts(2), &accounts(1), 1);
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
    let signed_delegate = dummy_signed_delegate(&accounts(2), &accounts(1), 1);
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
    let signed_delegate = dummy_signed_delegate(&accounts(2), &accounts(3), 1);
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
    testing_env!(context.build());
    let signed_delegate = dummy_signed_delegate(&accounts(2), &accounts(1), 1);
    let result = contract.relay_meta_transaction(signed_delegate);
    assert!(matches!(result, Err(RelayerError::ExpiredTransaction)));
}

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

#[test]
fn test_add_function_call_key() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();
    let mut public_key_bytes = vec![0];
    public_key_bytes.extend_from_slice(&[2; 32]);
    let public_key = PublicKey::try_from(public_key_bytes).unwrap();
    let result = contract.add_function_call_key(
        accounts(2),
        public_key,
        accounts(1),
        vec!["some_method".to_string()],
    );
    assert!(result.is_ok(), "Expected Ok, got {:?}", result);
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
    let result = contract.add_function_call_key(
        accounts(2),
        public_key,
        accounts(1),
        vec!["some_method".to_string()],
    );
    assert!(matches!(result, Err(RelayerError::Unauthorized)));
}

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
    let signed_delegate = dummy_signed_delegate(&accounts(2), &accounts(1), 1);
    let initial_gas = Gas::from_tgas(150);
    contract.set_simulate_promise_result(Some(SerializablePromiseResult::Failed));
    testing_env!(context.build());
    contract.callback_failure(signed_delegate.clone(), initial_gas);
    assert_eq!(contract.failed_transactions.len(), 0);
    assert_eq!(contract.processed_nonces.get(&AccountIdWrapper(WrappedAccountId(accounts(2)))), Some(&1));
    let logs = get_logs();
    assert!(logs.iter().any(|log| log.contains("Auto-retried transaction with nonce 1 succeeded")));
}

#[test]
fn test_callback_failure_auto_retry_insufficient_gas() {
    let (mut contract, mut context) = setup_contract();
    context.account_balance(NearToken::from_yoctonear(500_000_000_000_000_000_000_000));
    testing_env!(context.build());
    let signed_delegate = dummy_signed_delegate(&accounts(2), &accounts(1), 1);
    let initial_gas = Gas::from_tgas(150);
    contract.set_simulate_promise_result(Some(SerializablePromiseResult::Failed));
    testing_env!(context.build());
    contract.callback_failure(signed_delegate.clone(), initial_gas);
    assert_eq!(contract.failed_transactions.len(), 1);
    let (_, new_gas) = &contract.failed_transactions[0];
    assert_eq!(new_gas / 1_000_000_000_000, 230); // Updated from 180 to 230
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
    let signed_delegate = dummy_signed_delegate(&accounts(2), &accounts(1), 1);
    let initial_gas = Gas::from_tgas(150);
    contract.set_simulate_promise_result(Some(SerializablePromiseResult::Failed));
    testing_env!(context.build());
    contract.callback_failure(signed_delegate.clone(), initial_gas);
    assert_eq!(contract.failed_transactions.len(), 1);
    let (_, new_gas) = &contract.failed_transactions[0];
    assert_eq!(new_gas / 1_000_000_000_000, 230); // Updated from 180 to 230
    let logs = get_logs();
    assert!(logs.iter().any(|log| log.contains("Queued failed transaction with 230 TGas")));
}

#[test]
fn test_callback_failure_auto_retry_fails_queues() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();
    let signed_delegate = dummy_signed_delegate(&accounts(2), &accounts(1), 1);
    let initial_gas = Gas::from_tgas(150);
    contract.set_simulate_promise_result(Some(SerializablePromiseResult::Failed));
    contract.set_simulate_signature_failure(true);
    testing_env!(context.build());
    contract.callback_failure(signed_delegate.clone(), initial_gas);
    assert_eq!(contract.failed_transactions.len(), 1);
    let (_, new_gas) = &contract.failed_transactions[0];
    assert_eq!(new_gas / 1_000_000_000_000, 230); // Updated from 180 to 230
    let logs = get_logs();
    assert!(logs.iter().any(|log| log.contains("Queued failed transaction with 230 TGas")));
}

#[test]
fn test_callback_failure_multiple() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();
    let signed_delegate1 = dummy_signed_delegate(&accounts(2), &accounts(1), 1);
    let signed_delegate2 = dummy_signed_delegate(&accounts(2), &accounts(1), 2);
    let initial_gas = Gas::from_tgas(150);
    contract.set_simulate_promise_result(Some(SerializablePromiseResult::Failed));
    contract.set_simulate_signature_failure(true);
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
    let result = contract.relay_meta_transaction(signed_delegate);
    assert!(result.is_ok(), "Expected Ok, got {:?}", result);
}

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
    let result = contract.relay_meta_transaction(signed_delegate);
    assert!(matches!(result, Err(RelayerError::InvalidFTTransfer)));
}

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
    let result = contract.relay_meta_transaction(signed_delegate);
    assert!(matches!(result, Err(RelayerError::InsufficientDeposit)));
}

#[test]
fn test_sponsor_account_with_function_call_key() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();
    let mut public_key_bytes = vec![0];
    public_key_bytes.extend_from_slice(&[1; 32]);
    let public_key = PublicKey::try_from(public_key_bytes).unwrap();
    let result = contract.sponsor_account("user123".to_string(), public_key, true, false);
    assert!(result.is_ok());
}

#[test]
#[should_panic(expected = "overflow")]
fn test_gas_pool_overflow() {
    let (mut contract, mut context) = setup_contract();
    let max_deposit = NearToken::from_yoctonear(u128::MAX - contract.min_gas_pool + 1);
    context.attached_deposit(max_deposit);
    testing_env!(context.build());
    contract.deposit_gas_pool();
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
    let result = contract.relay_meta_transaction(signed_delegate);
    assert!(result.is_ok());
}

#[test]
fn test_relay_meta_transaction_valid_signature() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();
    let sender_id = accounts(2);
    context.signer_account_id(sender_id.clone());
    let signed_delegate = dummy_signed_delegate(&sender_id, &accounts(1), 1);
    contract.set_simulate_signature_failure(false);
    let result = contract.relay_meta_transaction(signed_delegate);
    assert!(result.is_ok(), "Expected Ok with valid signature, got {:?}", result);
    assert_eq!(contract.processed_nonces.get(&AccountIdWrapper(WrappedAccountId(sender_id))), Some(&1));
}

#[test]
fn test_relay_meta_transaction_invalid_signature() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();
    let sender_id = accounts(2);
    context.signer_account_id(sender_id.clone());
    let signed_delegate = dummy_signed_delegate(&sender_id, &accounts(1), 1);
    contract.set_simulate_signature_failure(true);
    let result = contract.relay_meta_transaction(signed_delegate);
    assert!(matches!(result, Err(RelayerError::InvalidSignature)));
    assert_eq!(contract.processed_nonces.get(&AccountIdWrapper(WrappedAccountId(sender_id))), None);
}

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
    let result = contract.relay_meta_transaction(signed_delegate);
    assert!(matches!(result, Err(RelayerError::InvalidFTTransfer)));
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
    let result = contract.sponsor_account("user123".to_string(), public_key, false, false);
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
    let result = contract.sponsor_account(invalid_implicit, public_key, false, true);
    assert!(matches!(result, Err(RelayerError::InvalidAccountId)));
}

#[test]
fn test_nonce_reuse_after_failure() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();
    let signed_delegate = dummy_signed_delegate(&accounts(2), &accounts(1), 1);
    contract.set_simulate_promise_result(Some(SerializablePromiseResult::Failed));
    contract.set_simulate_signature_failure(true);
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
    let signed_delegate = dummy_signed_delegate(&accounts(2), &accounts(1), u64::MAX);
    let result = contract.relay_meta_transaction(signed_delegate);
    assert!(result.is_ok());
}

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
    assert!(result.is_ok(), "Expected Ok, got {:?}", result);
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

#[test]
fn test_set_gas_config_success() {
    let (mut contract, _) = setup_contract();
    let default_gas_tgas = 200;
    let gas_buffer_tgas = 75;
    let result = contract.set_gas_config(default_gas_tgas, gas_buffer_tgas);
    assert!(result.is_ok(), "Expected Ok, got {:?}", result);
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

#[test]
fn test_retry_failed_transactions_manual_success() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();

    let signed_delegate = dummy_signed_delegate(&accounts(2), &accounts(1), 1);
    let initial_gas = Gas::from_tgas(150);
    contract.set_simulate_promise_result(Some(SerializablePromiseResult::Failed));
    contract.set_simulate_signature_failure(true);
    testing_env!(context.build());
    contract.callback_failure(signed_delegate.clone(), initial_gas);
    assert_eq!(contract.failed_transactions.len(), 1);

    contract.set_simulate_signature_failure(false);
    context.predecessor_account_id("onsocial.testnet".parse().unwrap());
    testing_env!(context.build());
    let result = contract.retry_or_clear_failed_transactions(true);
    assert!(result.is_ok(), "Expected Ok, got {:?}", result);
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

    let signed_delegate = dummy_signed_delegate(&accounts(2), &accounts(1), 1);
    let initial_gas = Gas::from_tgas(150);
    contract.set_simulate_promise_result(Some(SerializablePromiseResult::Failed));
    contract.set_simulate_signature_failure(true);
    testing_env!(context.build());
    contract.callback_failure(signed_delegate.clone(), initial_gas);
    assert_eq!(contract.failed_transactions.len(), 1);

    context.predecessor_account_id("onsocial.testnet".parse().unwrap());
    testing_env!(context.build());
    let result = contract.retry_or_clear_failed_transactions(true);
    assert!(result.is_ok(), "Expected Ok, got {:?}", result);
    assert_eq!(contract.failed_transactions.len(), 1);
    let (_, new_gas) = &contract.failed_transactions[0];
    assert_eq!(new_gas / 1_000_000_000_000, 300); // Updated from 230 to 300
    let logs = get_logs();
    assert!(logs.iter().any(|log| log.contains("Manual retry failed: InvalidSignature")));
}

#[test]
fn test_clear_failed_transactions_manual() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();

    let signed_delegate1 = dummy_signed_delegate(&accounts(2), &accounts(1), 1);
    let signed_delegate2 = dummy_signed_delegate(&accounts(2), &accounts(1), 2);
    let initial_gas = Gas::from_tgas(150);
    contract.set_simulate_promise_result(Some(SerializablePromiseResult::Failed));
    contract.set_simulate_signature_failure(true);
    testing_env!(context.build());
    contract.callback_failure(signed_delegate1, initial_gas);
    contract.callback_failure(signed_delegate2, initial_gas);
    assert_eq!(contract.failed_transactions.len(), 2);

    context.predecessor_account_id("onsocial.testnet".parse().unwrap());
    testing_env!(context.build());
    let result = contract.retry_or_clear_failed_transactions(false);
    assert!(result.is_ok(), "Expected Ok, got {:?}", result);
    assert_eq!(contract.failed_transactions.len(), 0);
    let logs = get_logs();
    assert!(logs.iter().any(|log| log.contains("failed_transactions_cleared")));
}

#[test]
fn test_retry_or_clear_no_failed_transactions() {
    let (mut contract, mut context) = setup_contract();
    context.predecessor_account_id("onsocial.testnet".parse().unwrap());
    testing_env!(context.build());
    let result = contract.retry_or_clear_failed_transactions(true);
    assert!(matches!(result, Err(RelayerError::NoFailedTransactions)));
    let result = contract.retry_or_clear_failed_transactions(false);
    assert!(matches!(result, Err(RelayerError::NoFailedTransactions)));
}

#[test]
fn test_retry_or_clear_unauthorized() {
    let (mut contract, mut context) = setup_contract();
    context.attached_deposit(NearToken::from_yoctonear(5_000_000_000_000_000_000_000_000));
    testing_env!(context.build());
    contract.deposit_gas_pool();

    let signed_delegate = dummy_signed_delegate(&accounts(2), &accounts(1), 1);
    let initial_gas = Gas::from_tgas(150);
    contract.set_simulate_promise_result(Some(SerializablePromiseResult::Failed));
    contract.set_simulate_signature_failure(true);
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
        let mut delegate = dummy_signed_delegate(&accounts(2), &accounts(1), i as u64);
        delegate.delegate_action.max_block_height = max_block_height;
        contract.failed_transactions.push((delegate, 150_000_000_000_000));
    }
    assert_eq!(contract.failed_transactions.len(), 150);

    context.block_height(1000);
    context.signer_account_id(accounts(2));
    testing_env!(context.build());
    let signed_delegate = dummy_signed_delegate(&accounts(2), &accounts(1), 151);
    let result = contract.relay_meta_transaction(signed_delegate);
    assert!(result.is_ok());

    assert_eq!(contract.failed_transactions.len(), crate::state::FAILED_TX_QUEUE_CAP as usize);
    for (signed_delegate, _) in contract.failed_transactions.iter() {
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
        let mut delegate = dummy_signed_delegate(&accounts(2), &accounts(1), i as u64);
        delegate.delegate_action.max_block_height = 1500;
        contract.failed_transactions.push((delegate, 150_000_000_000_000));
    }
    assert_eq!(contract.failed_transactions.len(), crate::state::FAILED_TX_QUEUE_CAP as usize);

    let signed_delegate = dummy_signed_delegate(&accounts(2), &accounts(1), crate::state::FAILED_TX_QUEUE_CAP as u64);
    contract.set_simulate_promise_result(Some(SerializablePromiseResult::Failed));
    contract.set_simulate_signature_failure(true);
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
        let mut delegate = dummy_signed_delegate(&accounts(2), &accounts(1), i as u64);
        delegate.delegate_action.max_block_height = max_block_height;
        contract.failed_transactions.push((delegate, 150_000_000_000_000));
    }
    assert_eq!(contract.failed_transactions.len(), 5);

    context.block_height(1000);
    context.predecessor_account_id("onsocial.testnet".parse().unwrap());
    contract.set_simulate_signature_failure(true);
    testing_env!(context.build());

    let result = contract.retry_or_clear_failed_transactions(true);
    assert!(result.is_ok());
    assert_eq!(contract.failed_transactions.len(), 2);
    for (signed_delegate, _) in contract.failed_transactions.iter() {
        assert!(signed_delegate.delegate_action.max_block_height >= 1000);
        assert!(signed_delegate.delegate_action.nonce >= 3);
    }
    let logs = get_logs();
    assert!(logs.iter().any(|log| log.contains("Manual retry failed")));
}