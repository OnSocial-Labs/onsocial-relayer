#[cfg(test)]
mod tests {
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

    // Initialization Tests
    #[test]
    fn test_new_contract() {
        let (contract, _) = setup_contract();
        assert_eq!(contract.relayer.admins, vec![accounts(0)]);
        assert_eq!(contract.relayer.auth_accounts.get(&accounts(1)).unwrap(), &"ed25519:6E8sCci9badyRkXb3JoRpBj5p8C19WVZw4BCrhmgbQHh".parse::<PublicKey>().unwrap());
        assert_eq!(contract.relayer.offload_recipient, accounts(2));
        assert_eq!(contract.relayer.gas_pool, 0);
    }

    #[test]
    #[should_panic(expected = "Use `new` to initialize")]
    fn test_default_panics() {
        let _ = OnSocialRelayer::default();
    }

    // Admin Tests
    #[test]
    fn test_add_auth_account_success() {
        let (mut contract, _) = setup_contract();
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
    fn test_remove_auth_account_non_existent() {
        let (mut contract, _) = setup_contract();
        let non_existent_account = accounts(3);

        let result = contract.remove_auth_account(non_existent_account.clone());
        assert!(result.is_ok(), "Removing non-existent auth account should succeed silently");
        assert!(contract.relayer.auth_accounts.get(&non_existent_account).is_none(), "Account should not exist");
    }

    #[test]
    fn test_set_offload_recipient_success() {
        let (mut contract, _) = setup_contract();
        let new_recipient = accounts(3);

        let result = contract.set_offload_recipient(new_recipient.clone());
        assert!(result.is_ok(), "Setting offload recipient should succeed");
        assert_eq!(contract.relayer.offload_recipient, new_recipient, "Offload recipient should be updated");
    }

    #[test]
    fn test_add_admin_success() {
        let (mut contract, _) = setup_contract();
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
        let (mut contract, _) = setup_contract();
        let existing_admin = accounts(0); // Already an admin

        let result = contract.add_admin(existing_admin.clone());
        assert!(result.is_ok(), "Adding duplicate admin should succeed but not change state");
        assert_eq!(contract.relayer.admins.len(), 1, "Admin list should not grow with duplicate");
    }

    #[test]
    fn test_remove_admin_success() {
        let (mut contract, _) = setup_contract();
        let new_admin = accounts(3);
        contract.add_admin(new_admin.clone()).unwrap();

        let result = contract.remove_admin(new_admin.clone());
        assert!(result.is_ok(), "Removing admin should succeed");
        assert!(!contract.relayer.admins.contains(&new_admin), "Admin should be removed");
        assert_eq!(contract.relayer.admins.len(), 1, "Admin list should have 1 entry");
    }

    #[test]
    fn test_remove_admin_last_admin() {
        let (mut contract, _) = setup_contract();
        let last_admin = accounts(0);

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
    fn test_remove_admin_non_existent() {
        let (mut contract, _) = setup_contract();
        let non_existent_admin = accounts(3);

        let result = contract.remove_admin(non_existent_admin);
        assert!(result.is_ok(), "Removing non-existent admin should succeed silently");
        assert_eq!(contract.relayer.admins.len(), 1, "Admin list should remain unchanged");
    }

    #[test]
    fn test_set_sponsor_amount_success() {
        let (mut contract, _) = setup_contract();
        let new_amount = 200_000_000_000_000_000_000_000_u128; // 0.2 NEAR

        let result = contract.set_sponsor_amount(new_amount.into());
        assert!(result.is_ok(), "Setting sponsor amount should succeed");
        assert_eq!(contract.relayer.sponsor_amount, new_amount, "Sponsor amount should be updated");
    }

    #[test]
    fn test_set_sponsor_amount_too_low() {
        let (mut contract, _) = setup_contract();
        let too_low_amount = 5_000_000_000_000_000_000_000_u128; // 0.005 NEAR

        let result = contract.set_sponsor_amount(too_low_amount.into());
        assert!(matches!(result, Err(RelayerError::AmountTooLow)), "Should fail when amount too low");
        assert_eq!(contract.relayer.sponsor_amount, 100_000_000_000_000_000_000_000, "Sponsor amount should not change");
    }

    #[test]
    fn test_set_sponsor_amount_unauthorized() {
        let (mut contract, mut context) = setup_contract();
        context.predecessor_account_id(accounts(3)); // Non-admin
        testing_env!(context.build());

        let new_amount = 200_000_000_000_000_000_000_000_u128; // 0.2 NEAR

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

    #[test]
    fn test_deposit_gas_pool_zero() {
        let (mut contract, mut context) = setup_contract();
        context.attached_deposit(NearToken::from_near(0));
        testing_env!(context.build());

        let result = contract.deposit_gas_pool();
        assert!(result.is_ok(), "Zero deposit should succeed");
        assert_eq!(contract.get_gas_pool().0, 0, "Gas pool should remain unchanged");
    }

    #[test]
    fn test_deposit_gas_pool_at_max() {
        let (mut contract, mut context) = setup_contract();
        let max_gas_pool = contract.relayer.max_gas_pool;
        context.attached_deposit(NearToken::from_yoctonear(max_gas_pool));
        testing_env!(context.build());

        let result = contract.deposit_gas_pool();
        assert!(result.is_ok(), "Deposit at max should succeed");
        assert_eq!(contract.get_gas_pool().0, max_gas_pool, "Gas pool should equal max");
    }

    #[test]
    fn test_deposit_direct_success() {
        let (mut contract, mut context) = setup_contract();
        context.attached_deposit(NearToken::from_near(5));
        testing_env!(context.build());

        contract.deposit();
        assert_eq!(
            contract.get_gas_pool().0,
            5_000_000_000_000_000_000_000_000,
            "Gas pool should increase by direct deposit"
        );
    }

    #[test]
    fn test_deposit_direct_exceeds_max() {
        let (mut contract, mut context) = setup_contract();
        context.attached_deposit(NearToken::from_near(600));
        testing_env!(context.build());

        contract.deposit();
        assert_eq!(
            contract.get_gas_pool().0,
            contract.relayer.max_gas_pool,
            "Gas pool should cap at max with excess offloaded"
        );
    }

    // Relay Tests
    #[test]
    fn test_relay_meta_transaction_transfer_success() {
        let (mut contract, mut context) = setup_contract();
        context.attached_deposit(NearToken::from_near(10));
        testing_env!(context.build());
        contract.deposit_gas_pool().unwrap();

        let signed_delegate = create_signed_delegate(
            accounts(1),
            accounts(2),
            vec![Action::Transfer { deposit: NearToken::from_near(1) }],
        );

        let result = contract.relay_meta_transaction(signed_delegate);
        assert!(result.is_ok(), "Relay should succeed");
        assert_eq!(contract.get_gas_pool().0, 9_971_000_000_000_000_000_000_000, "Gas pool should decrease by 0.029 NEAR");
    }

    #[test]
    fn test_relay_meta_transaction_function_call_success() {
        let (mut contract, mut context) = setup_contract();
        context.attached_deposit(NearToken::from_near(10));
        testing_env!(context.build());
        contract.deposit_gas_pool().unwrap();

        let signed_delegate = create_signed_delegate(
            accounts(1),
            accounts(2),
            vec![Action::FunctionCall {
                method_name: "some_method".to_string(),
                args: vec![1, 2, 3],
                gas: Gas::from_tgas(50),
                deposit: NearToken::from_near(0),
            }],
        );

        let result = contract.relay_meta_transaction(signed_delegate);
        assert!(result.is_ok(), "Relay should succeed");
        assert_eq!(contract.get_gas_pool().0, 9_971_000_000_000_000_000_000_000, "Gas pool should decrease by 0.029 NEAR");
    }

    #[test]
    fn test_relay_meta_transaction_add_key_success() {
        let (mut contract, mut context) = setup_contract();
        context.attached_deposit(NearToken::from_near(10));
        testing_env!(context.build());
        contract.deposit_gas_pool().unwrap();

        let signed_delegate = create_signed_delegate(
            accounts(1),
            accounts(2),
            vec![Action::AddKey {
                public_key: "ed25519:8fWHEecB2iXjZ75kMYG34M2DSELK9nQ31K3vQ3Wy4nqW".parse().unwrap(),
                allowance: Some(NearToken::from_near(1)),
                receiver_id: accounts(2),
                method_names: vec!["method1".to_string()],
            }],
        );

        let result = contract.relay_meta_transaction(signed_delegate);
        assert!(result.is_ok(), "Relay should succeed");
        assert_eq!(contract.get_gas_pool().0, 9_971_000_000_000_000_000_000_000, "Gas pool should decrease by 0.029 NEAR");
    }

    #[test]
    fn test_relay_meta_transaction_chain_signature_success() {
        let (mut contract, mut context) = setup_contract();
        context.attached_deposit(NearToken::from_near(10));
        context.prepaid_gas(Gas::from_tgas(350)); // Increased beyond 300 TGas due to mock environment overhead; real blockchain should fit within 300 TGas
        testing_env!(context.build());
        contract.deposit_gas_pool().unwrap();

        let signed_delegate = create_signed_delegate(
            accounts(1),
            accounts(2),
            vec![Action::ChainSignatureRequest {
                target_chain: "mpc.near".to_string(),
                derivation_path: "1".to_string(),
                payload: vec![1, 2, 3],
            }],
        );

        let result = contract.relay_meta_transaction(signed_delegate);
        assert!(result.is_ok(), "Relay should succeed");
        assert_eq!(contract.get_gas_pool().0, 9_971_000_000_000_000_000_000_000, "Gas pool should decrease by 0.029 NEAR");
    }

    #[test]
    fn test_relay_meta_transaction_insufficient_gas() {
        let (mut contract, _) = setup_contract();
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
            accounts(3), // Unauthorized
            accounts(2),
            vec![Action::Transfer { deposit: NearToken::from_near(1) }],
        );

        let result = contract.relay_meta_transaction(signed_delegate);
        assert!(matches!(result, Err(RelayerError::Unauthorized)), "Should fail due to unauthorized sender");
    }

    #[test]
    fn test_relay_meta_transaction_gas_at_min() {
    let (mut contract, mut context) = setup_contract();
    let min_plus_tx_cost = contract.relayer.min_gas_pool + 29_000_000_000_000_000_000_000_u128; // 1 NEAR + 0.029 NEAR
    context.attached_deposit(NearToken::from_yoctonear(min_plus_tx_cost));
    testing_env!(context.build());
    contract.deposit_gas_pool().unwrap();

    let signed_delegate = create_signed_delegate(
        accounts(1),
        accounts(2),
        vec![Action::Transfer { deposit: NearToken::from_near(0) }],
    );

    let result = contract.relay_meta_transaction(signed_delegate);
    assert!(result.is_ok(), "Relay should succeed at min gas pool plus tx cost");
    assert_eq!(
        contract.get_gas_pool().0,
        contract.relayer.min_gas_pool, // After tx, gas_pool should equal min_gas_pool
        "Gas pool should decrease by 0.029 NEAR to min_gas_pool"
    );
    }

    #[test]
    fn test_relay_meta_transaction_invalid_chain_signature() {
        let (mut contract, mut context) = setup_contract();
        context.attached_deposit(NearToken::from_near(10));
        testing_env!(context.build());
        contract.deposit_gas_pool().unwrap();

        let signed_delegate = create_signed_delegate(
            accounts(1),
            accounts(2),
            vec![Action::ChainSignatureRequest {
                target_chain: "#invalid".to_string(), // Invalid AccountId
                derivation_path: "1".to_string(),
                payload: vec![1, 2, 3],
            }],
        );

        let result = contract.relay_meta_transaction(signed_delegate);
        assert!(matches!(result, Err(RelayerError::InvalidAccountId)), "Should fail due to invalid target chain");
    }

    #[test]
    fn test_relay_meta_transactions_batch_success() {
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
        assert_eq!(contract.get_gas_pool().0, 19_942_000_000_000_000_000_000_000, "Gas pool should decrease by 0.058 NEAR");
    }

    #[test]
    fn test_relay_meta_transactions_batch_insufficient_gas() {
        let (mut contract, mut context) = setup_contract();
        context.attached_deposit(NearToken::from_near(1)); // Less than 2x min_gas_pool
        testing_env!(context.build());
        contract.deposit_gas_pool().unwrap();

        let signed_delegates = vec![
            create_signed_delegate(accounts(1), accounts(2), vec![Action::Transfer { deposit: NearToken::from_near(1) }]),
            create_signed_delegate(accounts(1), accounts(3), vec![Action::Transfer { deposit: NearToken::from_near(1) }]),
        ];

        let result = contract.relay_meta_transactions(signed_delegates);
        assert!(matches!(result, Err(RelayerError::InsufficientGasPool)), "Should fail due to insufficient gas for batch");
    }

    #[test]
    fn test_relay_meta_transactions_batch_empty() {
        let (mut contract, mut context) = setup_contract();
        context.attached_deposit(NearToken::from_near(10));
        testing_env!(context.build());
        contract.deposit_gas_pool().unwrap();

        let signed_delegates: Vec<SignedDelegateAction> = vec![];

        let result = contract.relay_meta_transactions(signed_delegates);
        assert!(matches!(result, Err(RelayerError::InvalidNonce)), "Should fail due to empty batch");
    }

    #[test]
    fn test_relay_meta_transactions_exceed_chunk_size() {
        let (mut contract, mut context) = setup_contract();
        context.attached_deposit(NearToken::from_near(20));
        testing_env!(context.build());
        contract.deposit_gas_pool().unwrap();

        let signed_delegates = vec![
            create_signed_delegate(accounts(1), accounts(2), vec![Action::Transfer { deposit: NearToken::from_near(1) }]),
            create_signed_delegate(accounts(1), accounts(2), vec![Action::Transfer { deposit: NearToken::from_near(1) }]),
            create_signed_delegate(accounts(1), accounts(2), vec![Action::Transfer { deposit: NearToken::from_near(1) }]),
            create_signed_delegate(accounts(1), accounts(2), vec![Action::Transfer { deposit: NearToken::from_near(1) }]),
            create_signed_delegate(accounts(1), accounts(2), vec![Action::Transfer { deposit: NearToken::from_near(1) }]),
            create_signed_delegate(accounts(1), accounts(2), vec![Action::Transfer { deposit: NearToken::from_near(1) }]), // 6 exceeds CHUNK_SIZE (5)
        ];

        let result = contract.relay_meta_transactions(signed_delegates);
        assert!(matches!(result, Err(RelayerError::InvalidNonce)), "Should fail due to exceeding chunk size");
    }

    #[test]
    fn test_relay_chunked_meta_transactions_multi_chunk_success() {
        let (mut contract, mut context) = setup_contract();
        context.attached_deposit(NearToken::from_near(20));
        testing_env!(context.build());
        contract.deposit_gas_pool().unwrap();

        let signed_delegates = vec![
            create_signed_delegate(accounts(1), accounts(2), vec![Action::Transfer { deposit: NearToken::from_near(1) }]),
            create_signed_delegate(accounts(1), accounts(3), vec![Action::Transfer { deposit: NearToken::from_near(1) }]),
            create_signed_delegate(accounts(1), accounts(4), vec![Action::Transfer { deposit: NearToken::from_near(1) }]),
            create_signed_delegate(accounts(1), accounts(5), vec![Action::Transfer { deposit: NearToken::from_near(1) }]),
            create_signed_delegate(accounts(1), accounts(2), vec![Action::Transfer { deposit: NearToken::from_near(1) }]),
            create_signed_delegate(accounts(1), accounts(3), vec![Action::Transfer { deposit: NearToken::from_near(1) }]), // 6 txs, 2 chunks
        ];

        let result = contract.relay_chunked_meta_transactions(signed_delegates);
        assert!(result.is_ok(), "Chunked relay should succeed");
        let promises = result.unwrap();
        assert_eq!(promises.len(), 6, "Should return six promises");
        assert_eq!(contract.get_gas_pool().0, 19_826_000_000_000_000_000_000_000, "Gas pool should decrease by 0.174 NEAR (6 * 0.029)");
    }

    #[test]
    fn test_relay_chunked_meta_transactions_empty() {
        let (mut contract, mut context) = setup_contract();
        context.attached_deposit(NearToken::from_near(10));
        testing_env!(context.build());
        contract.deposit_gas_pool().unwrap();

        let signed_delegates: Vec<SignedDelegateAction> = vec![];

        let result = contract.relay_chunked_meta_transactions(signed_delegates);
        assert!(matches!(result, Err(RelayerError::InvalidNonce)), "Should fail due to empty chunked batch");
    }

    #[test]
    fn test_relay_chunked_meta_transactions_insufficient_gas() {
        let (mut contract, mut context) = setup_contract();
        context.attached_deposit(NearToken::from_millinear(100)); // 0.1 NEAR, below min_gas_pool (1 NEAR) + 6 * 0.029 NEAR
        testing_env!(context.build());
        contract.deposit_gas_pool().unwrap();

        let signed_delegates = vec![
            create_signed_delegate(accounts(1), accounts(2), vec![Action::Transfer { deposit: NearToken::from_near(1) }]),
            create_signed_delegate(accounts(1), accounts(3), vec![Action::Transfer { deposit: NearToken::from_near(1) }]),
            create_signed_delegate(accounts(1), accounts(4), vec![Action::Transfer { deposit: NearToken::from_near(1) }]),
            create_signed_delegate(accounts(1), accounts(5), vec![Action::Transfer { deposit: NearToken::from_near(1) }]),
            create_signed_delegate(accounts(1), accounts(2), vec![Action::Transfer { deposit: NearToken::from_near(1) }]),
            create_signed_delegate(accounts(1), accounts(3), vec![Action::Transfer { deposit: NearToken::from_near(1) }]),
        ];

        let result = contract.relay_chunked_meta_transactions(signed_delegates);
        assert!(matches!(result, Err(RelayerError::InsufficientGasPool)), "Should fail due to insufficient gas for chunked batch");
    }

    // Gas Refund Callback Tests
    #[test]
    fn test_refund_gas_callback_simple() {
        let (mut contract, mut context) = setup_contract();
        context.attached_deposit(NearToken::from_near(10));
        context.prepaid_gas(Gas::from_tgas(290));
        testing_env!(context.build());
        contract.deposit_gas_pool().unwrap();

        let signed_delegate = create_signed_delegate(
            accounts(1),
            accounts(2),
            vec![Action::Transfer { deposit: NearToken::from_near(0) }],
        );
        let _ = contract.relay_meta_transaction(signed_delegate);
        assert_eq!(
            contract.get_gas_pool().0,
            9_971_000_000_000_000_000_000_000,
            "Gas pool should reflect full initial cost deduction (refund not mocked in unit test)"
        );
    }

    #[test]
    fn test_refund_gas_callback_with_overflow() {
        let (mut contract, mut context) = setup_contract();
        context.attached_deposit(NearToken::from_near(600));
        context.prepaid_gas(Gas::from_tgas(290));
        testing_env!(context.build());
        contract.deposit_gas_pool().unwrap();

        let signed_delegate = create_signed_delegate(
            accounts(1),
            accounts(2),
            vec![Action::Transfer { deposit: NearToken::from_near(0) }],
        );
        let _ = contract.relay_meta_transaction(signed_delegate);
        assert_eq!(
            contract.get_gas_pool().0,
            contract.relayer.max_gas_pool - 29_000_000_000_000_000_000_000,
            "Gas pool should decrease by initial cost, capped at max"
        );
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
        let (mut contract, _) = setup_contract();
        let new_account_name = "testuser".to_string();
        let public_key: PublicKey = "ed25519:8fWHEecB2iXjZ75kMYG34M2DSELK9nQ31K3vQ3Wy4nqW".parse().unwrap();

        let result = contract.sponsor_account(new_account_name, public_key);
        assert!(matches!(result, Err(RelayerError::InsufficientGasPool)), "Should fail due to insufficient gas");
    }

    #[test]
    fn test_sponsor_account_exact_gas() {
        let (mut contract, mut context) = setup_contract();
        let min_plus_sponsor = contract.relayer.min_gas_pool + contract.relayer.sponsor_amount;
        context.attached_deposit(NearToken::from_yoctonear(min_plus_sponsor));
        testing_env!(context.build());
        contract.deposit_gas_pool().unwrap();

        let new_account_name = "testuser".to_string();
        let public_key: PublicKey = "ed25519:8fWHEecB2iXjZ75kMYG34M2DSELK9nQ31K3vQ3Wy4nqW".parse().unwrap();

        let result = contract.sponsor_account(new_account_name, public_key);
        assert!(result.is_ok(), "Sponsoring should succeed at exact threshold");
        assert_eq!(contract.get_gas_pool().0, contract.relayer.min_gas_pool, "Gas pool should be at min after sponsor");
    }

    #[test]
    fn test_sponsor_account_invalid_name() {
        let (mut contract, mut context) = setup_contract();
        context.attached_deposit(NearToken::from_near(10));
        testing_env!(context.build());
        contract.deposit_gas_pool().unwrap();

        let invalid_name = "".to_string(); // Empty name
        let public_key: PublicKey = "ed25519:8fWHEecB2iXjZ75kMYG34M2DSELK9nQ31K3vQ3Wy4nqW".parse().unwrap();

        let result = contract.sponsor_account(invalid_name, public_key);
        assert!(matches!(result, Err(RelayerError::InvalidAccountId)), "Should fail due to invalid account name");
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
}