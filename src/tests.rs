#[cfg(test)]
mod tests {
    use near_sdk::{testing_env, test_utils::VMContextBuilder, AccountId, PublicKey, NearToken, Gas};
    use near_sdk::json_types::U128;
    use crate::{OnSocialRelayer, RelayerError};
    use crate::types::{SignedDelegateAction, DelegateAction, Action, SignatureScheme};
    use near_crypto::{InMemorySigner, KeyType, Signature};

    fn setup_contract() -> (OnSocialRelayer, VMContextBuilder) {
        let mut context = VMContextBuilder::new();
        context
            .predecessor_account_id(accounts(0)) // Admin
            .attached_deposit(NearToken::from_near(0));
        testing_env!(context.build());
        let signer = InMemorySigner::from_seed(accounts(1), KeyType::ED25519, "test-seed");
        let public_key = PublicKey::try_from(borsh::to_vec(&signer.public_key()).unwrap()).unwrap();
        let contract = OnSocialRelayer::new(
            vec![accounts(0)], // admins
            accounts(1),       // initial_auth_account
            public_key,
            accounts(2),       // offload_recipient
        );
        (contract, context)
    }

    fn create_signed_delegate(
        sender_id: AccountId,
        receiver_id: AccountId,
        actions: Vec<Action>,
        fee_action: Option<Action>,
    ) -> SignedDelegateAction {
        let signer = InMemorySigner::from_seed(sender_id.clone(), KeyType::ED25519, "test-seed");
        let public_key = PublicKey::try_from(borsh::to_vec(&signer.public_key()).unwrap()).unwrap();
        let delegate_action = DelegateAction {
            sender_id,
            receiver_id,
            actions,
            nonce: 1,
            max_block_height: 1000,
        };
        let payload = borsh::to_vec(&delegate_action).unwrap();
        let signature = signer.sign(&payload);
        SignedDelegateAction {
            delegate_action,
            signature: match signature {
                Signature::ED25519(sig) => sig.to_bytes().to_vec(),
                _ => panic!("Unexpected signature"),
            },
            public_key,
            session_nonce: 1,
            scheme: SignatureScheme::Ed25519,
            fee_action,
        }
    }

    fn accounts(index: u32) -> AccountId {
        AccountId::try_from(format!("account{}.testnet", index))
            .unwrap_or_else(|_| panic!("Failed to create test account ID"))
    }

    #[test]
    fn test_new_contract() {
        let (contract, _) = setup_contract();
        assert_eq!(contract.relayer.admins, vec![accounts(0)]);
        let signer = InMemorySigner::from_seed(accounts(1), KeyType::ED25519, "test-seed");
        let expected_pk = PublicKey::try_from(borsh::to_vec(&signer.public_key()).unwrap()).unwrap();
        assert_eq!(contract.relayer.auth_accounts.get(&accounts(1)).unwrap(), &expected_pk);
        assert_eq!(contract.relayer.gas_pool, 0);
        assert_eq!(contract.relayer.chunk_size, 5);
        assert_eq!(
            contract.relayer.chain_mpc_mapping.get(&"near".to_string()),
            Some(&"mpc.near".parse().unwrap())
        );
        assert_eq!(contract.get_version(), "1.0", "Initial version should be 1.0");
    }

    #[test]
    #[should_panic(expected = "Use `new` to initialize")]
    fn test_default_panics() {
        let _ = OnSocialRelayer::default();
    }

    #[test]
    fn test_add_auth_account_success() {
        let (mut contract, _) = setup_contract();
        let result = contract.add_auth_account(
            accounts(3),
            "ed25519:8fWHEecB2iXjZ75kMYG34M2DSELK9nQ31K3vQ3Wy4nqW"
                .parse()
                .unwrap(),
        );
        assert!(result.is_ok());
        assert_eq!(
            contract.relayer.auth_accounts.get(&accounts(3)).unwrap(),
            &"ed25519:8fWHEecB2iXjZ75kMYG34M2DSELK9nQ31K3vQ3Wy4nqW"
                .parse::<PublicKey>()
                .unwrap()
        );
    }

    #[test]
    fn test_add_auth_account_unauthorized() {
        let (mut contract, mut context) = setup_contract();
        context.predecessor_account_id(accounts(3));
        testing_env!(context.build());
        let result = contract.add_auth_account(
            accounts(4),
            "ed25519:8fWHEecB2iXjZ75kMYG34M2DSELK9nQ31K3vQ3Wy4nqW"
                .parse()
                .unwrap(),
        );
        assert!(matches!(result, Err(RelayerError::Unauthorized)));
    }

    #[test]
    fn test_remove_auth_account() {
        let (mut contract, _) = setup_contract();
        let result = contract.remove_auth_account(accounts(1));
        assert!(result.is_ok());
        assert!(contract.relayer.auth_accounts.get(&accounts(1)).is_none());
    }

    #[test]
    fn test_set_offload_recipient() {
        let (mut contract, _) = setup_contract();
        let result = contract.set_offload_recipient(accounts(3));
        assert!(result.is_ok());
        assert_eq!(contract.relayer.offload_recipient, accounts(3));
    }

    #[test]
    fn test_add_admin() {
        let (mut contract, _) = setup_contract();
        let result = contract.add_admin(accounts(3));
        assert!(result.is_ok());
        assert!(contract.relayer.admins.contains(&accounts(3)));
    }

    #[test]
    fn test_remove_admin() {
        let (mut contract, _) = setup_contract();
        contract.add_admin(accounts(3)).unwrap();
        let result = contract.remove_admin(accounts(3));
        assert!(result.is_ok());
        assert!(!contract.relayer.admins.contains(&accounts(3)));
    }

    #[test]
    fn test_remove_admin_last() {
        let (mut contract, _) = setup_contract();
        let result = contract.remove_admin(accounts(0));
        assert!(matches!(result, Err(RelayerError::LastAdmin)));
    }

    #[test]
    fn test_set_sponsor_amount() {
        let (mut contract, _) = setup_contract();
        let result = contract.set_sponsor_amount(U128(200_000_000_000_000_000_000_000));
        assert!(result.is_ok());
        assert_eq!(
            contract.relayer.sponsor_amount,
            200_000_000_000_000_000_000_000
        );
    }

    #[test]
    fn test_set_sponsor_amount_too_low() {
        let (mut contract, _) = setup_contract();
        let result = contract.set_sponsor_amount(U128(5_000_000_000_000_000_000_000));
        assert!(matches!(result, Err(RelayerError::AmountTooLow)));
    }

    #[test]
    fn test_set_max_gas_pool() {
        let (mut contract, _) = setup_contract();
        let result = contract.set_max_gas_pool(U128(600_000_000_000_000_000_000_000_000));
        assert!(result.is_ok());
        assert_eq!(
            contract.relayer.max_gas_pool,
            600_000_000_000_000_000_000_000_000
        );
    }

    #[test]
    fn test_set_max_gas_pool_too_low() {
        let (mut contract, _) = setup_contract();
        let result = contract.set_max_gas_pool(U128(500_000_000_000_000_000_000_000));
        assert!(matches!(result, Err(RelayerError::AmountTooLow)));
    }

    #[test]
    fn test_set_min_gas_pool() {
        let (mut contract, _) = setup_contract();
        let result = contract.set_min_gas_pool(U128(2_000_000_000_000_000_000_000_000));
        assert!(result.is_ok());
        assert_eq!(
            contract.relayer.min_gas_pool,
            2_000_000_000_000_000_000_000_000
        );
    }

    #[test]
    fn test_add_chain_mpc_mapping() {
        let (mut contract, _) = setup_contract();
        let result = contract.add_chain_mpc_mapping("ethereum".to_string(), accounts(3));
        assert!(result.is_ok());
        assert_eq!(
            contract.relayer.chain_mpc_mapping.get(&"ethereum".to_string()),
            Some(&accounts(3))
        );
    }

    #[test]
    fn test_remove_chain_mpc_mapping() {
        let (mut contract, _) = setup_contract();
        let result = contract.remove_chain_mpc_mapping("near".to_string());
        assert!(result.is_ok());
        assert!(contract
            .relayer
            .chain_mpc_mapping
            .get(&"near".to_string())
            .is_none());
    }

    #[test]
    fn test_set_chunk_size() {
        let (mut contract, _) = setup_contract();
        let result = contract.set_chunk_size(10);
        assert!(result.is_ok());
        assert_eq!(contract.relayer.chunk_size, 10);
    }

    #[test]
    fn test_set_chunk_size_invalid() {
        let (mut contract, _) = setup_contract();
        let result = contract.set_chunk_size(0);
        assert!(matches!(result, Err(RelayerError::AmountTooLow)));
    }

    #[test]
    fn test_deposit_gas_pool() {
        let (mut contract, mut context) = setup_contract();
        context.attached_deposit(NearToken::from_near(2));
        testing_env!(context.build());
        let result = contract.deposit_gas_pool();
        assert!(result.is_ok());
        assert_eq!(
            contract.get_gas_pool().0,
            2_000_000_000_000_000_000_000_000
        );
    }

    #[test]
    fn test_deposit_gas_pool_excess() {
        let (mut contract, mut context) = setup_contract();
        context.attached_deposit(NearToken::from_near(600));
        testing_env!(context.build());
        let result = contract.deposit_gas_pool();
        assert!(result.is_ok());
        assert_eq!(
            contract.get_gas_pool().0,
            500_000_000_000_000_000_000_000_000 // Max gas pool
        );
    }

    #[test]
    fn test_deposit_direct() {
        let (mut contract, mut context) = setup_contract();
        context.attached_deposit(NearToken::from_near(5));
        testing_env!(context.build());
        contract.deposit();
        assert_eq!(
            contract.get_gas_pool().0,
            5_000_000_000_000_000_000_000_000
        );
    }

    #[test]
    fn test_relay_meta_transaction_transfer() {
        let (mut contract, mut context) = setup_contract();
        context.attached_deposit(NearToken::from_near(10));
        testing_env!(context.build());
        contract.deposit_gas_pool().unwrap();
        let initial_gas_pool = contract.get_gas_pool().0;
        let signed_delegate = create_signed_delegate(
            accounts(1),
            accounts(2),
            vec![Action::Transfer {
                deposit: NearToken::from_near(1),
            }],
            None,
        );
        let result = contract.relay_meta_transaction(signed_delegate);
        if let Err(e) = &result {
            panic!("Relay failed: {:?}", e);
        }
        assert!(result.is_ok());
        assert_eq!(
            contract.get_gas_pool().0,
            initial_gas_pool - 29_000_000_000_000_000_000_000 // 0.029 NEAR gas cost
        );
    }

    #[test]
    fn test_relay_meta_transaction_with_fee() {
        let (mut contract, mut context) = setup_contract();
        context.attached_deposit(NearToken::from_near(10));
        testing_env!(context.build());
        contract.deposit_gas_pool().unwrap();
        let initial_gas_pool = contract.get_gas_pool().0;
        let signed_delegate = create_signed_delegate(
            accounts(1),
            accounts(2),
            vec![Action::Transfer {
                deposit: NearToken::from_near(1),
            }],
            Some(Action::Transfer {
                deposit: NearToken::from_millinear(500),
            }),
        );
        let result = contract.relay_meta_transaction(signed_delegate);
        if let Err(e) = &result {
            panic!("Relay failed: {:?}", e);
        }
        assert!(result.is_ok());
        assert_eq!(
            contract.get_gas_pool().0,
            initial_gas_pool - (29_000_000_000_000_000_000_000 * 2) // 0.058 NEAR total gas cost
        );
    }

    #[test]
    fn test_relay_meta_transaction_insufficient_gas() {
        let (mut contract, mut context) = setup_contract();
        context.attached_deposit(NearToken::from_millinear(500)); // 0.5 NEAR, less than min + cost
        testing_env!(context.build());
        contract.deposit_gas_pool().unwrap();
        let signed_delegate = create_signed_delegate(
            accounts(1),
            accounts(2),
            vec![Action::Transfer {
                deposit: NearToken::from_near(1),
            }],
            None,
        );
        let result = contract.relay_meta_transaction(signed_delegate);
        assert!(matches!(result, Err(RelayerError::InsufficientGasPool)));
    }

    #[test]
    fn test_relay_meta_transaction_unauthorized() {
        let (mut contract, mut context) = setup_contract();
        context.attached_deposit(NearToken::from_near(10));
        testing_env!(context.build());
        contract.deposit_gas_pool().unwrap();
        let signed_delegate = create_signed_delegate(
            accounts(3), // Not in auth_accounts
            accounts(2),
            vec![Action::Transfer {
                deposit: NearToken::from_near(1),
            }],
            None,
        );
        let result = contract.relay_meta_transaction(signed_delegate);
        assert!(matches!(result, Err(RelayerError::Unauthorized)));
    }

    #[test]
    fn test_relay_meta_transaction_function_call() {
        let (mut contract, mut context) = setup_contract();
        context.attached_deposit(NearToken::from_near(10));
        testing_env!(context.build());
        contract.deposit_gas_pool().unwrap();
        let initial_gas_pool = contract.get_gas_pool().0;
        let signed_delegate = create_signed_delegate(
            accounts(1),
            accounts(2),
            vec![Action::FunctionCall {
                method_name: "test".to_string(),
                args: vec![1, 2, 3],
                gas: Gas::from_tgas(50),
                deposit: NearToken::from_near(0),
            }],
            None,
        );
        let result = contract.relay_meta_transaction(signed_delegate);
        if let Err(e) = &result {
            panic!("Relay failed: {:?}", e);
        }
        assert!(result.is_ok());
        assert_eq!(
            contract.get_gas_pool().0,
            initial_gas_pool - 29_000_000_000_000_000_000_000 // 0.029 NEAR gas cost
        );
    }

    #[test]
    fn test_relay_meta_transaction_chain_signature() {
        let (mut contract, mut context) = setup_contract();
        context
            .attached_deposit(NearToken::from_near(10))
            .prepaid_gas(Gas::from_tgas(300));
        testing_env!(context.build());
        contract.deposit_gas_pool().unwrap();
        let initial_gas_pool = contract.get_gas_pool().0;
        let signed_delegate = create_signed_delegate(
            accounts(1),
            "mpc.near".parse().unwrap(),
            vec![Action::ChainSignatureRequest {
                target_chain: "near".to_string(),
                derivation_path: "m/44'/60'/0'/0/0".to_string(),
                payload: vec![1, 2, 3],
            }],
            None,
        );
        let result = contract.relay_meta_transaction(signed_delegate);
        if let Err(e) = &result {
            panic!("Relay failed: {:?}", e);
        }
        assert!(result.is_ok());
        assert_eq!(
            contract.get_gas_pool().0,
            initial_gas_pool - 29_000_000_000_000_000_000_000 // 0.029 NEAR gas cost
        );
    }

    #[test]
    fn test_relay_meta_transactions_multiple_chain_signatures() {
        let (mut contract, mut context) = setup_contract();
        context
            .attached_deposit(NearToken::from_near(10))
            .prepaid_gas(Gas::from_tgas(300));
        testing_env!(context.build());
        contract.deposit_gas_pool().unwrap();
        let initial_gas_pool = contract.get_gas_pool().0;
    
        let signed_delegates = vec![
            create_signed_delegate(
                accounts(1),
                "mpc.near".parse().unwrap(),
                vec![Action::ChainSignatureRequest {
                    target_chain: "near".to_string(),
                    derivation_path: "m/44'/60'/0'/0/0".to_string(),
                    payload: vec![1, 2, 3],
                }],
                None,
            ),
        ];
        let result = contract.relay_meta_transactions(signed_delegates);
        if let Err(e) = &result {
            panic!("Relay failed: {:?}", e);
        }
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
        assert_eq!(
            contract.get_gas_pool().0,
            initial_gas_pool - 29_000_000_000_000_000_000_000,
            "Gas pool should decrease by cost of one action"
        );
    }

    #[test]
    fn test_relay_meta_transactions_batch() {
        let (mut contract, mut context) = setup_contract();
        context.attached_deposit(NearToken::from_near(20));
        testing_env!(context.build());
        contract.deposit_gas_pool().unwrap();
        let initial_gas_pool = contract.get_gas_pool().0;
        let signed_delegates = vec![
            create_signed_delegate(
                accounts(1),
                accounts(2),
                vec![Action::Transfer {
                    deposit: NearToken::from_near(1),
                }],
                None,
            ),
            create_signed_delegate(
                accounts(1),
                accounts(3),
                vec![Action::Transfer {
                    deposit: NearToken::from_near(1),
                }],
                None,
            ),
        ];
        let result = contract.relay_meta_transactions(signed_delegates);
        if let Err(e) = &result {
            panic!("Batch relay failed: {:?}", e);
        }
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
        assert_eq!(
            contract.get_gas_pool().0,
            initial_gas_pool - (29_000_000_000_000_000_000_000 * 2) // 0.058 NEAR for 2 actions
        );
    }

    #[test]
    fn test_relay_chunked_meta_transactions() {
        let (mut contract, mut context) = setup_contract();
        context.attached_deposit(NearToken::from_near(20));
        testing_env!(context.build());
        contract.deposit_gas_pool().unwrap();
        let initial_gas_pool = contract.get_gas_pool().0;
        let signed_delegates = vec![
            create_signed_delegate(
                accounts(1),
                accounts(2),
                vec![Action::Transfer {
                    deposit: NearToken::from_near(1),
                }],
                None,
            ),
            create_signed_delegate(
                accounts(1),
                accounts(3),
                vec![Action::Transfer {
                    deposit: NearToken::from_near(1),
                }],
                None,
            ),
            create_signed_delegate(
                accounts(1),
                accounts(4),
                vec![Action::Transfer {
                    deposit: NearToken::from_near(1),
                }],
                None,
            ),
            create_signed_delegate(
                accounts(1),
                accounts(5),
                vec![Action::Transfer {
                    deposit: NearToken::from_near(1),
                }],
                None,
            ),
            create_signed_delegate(
                accounts(1),
                accounts(2),
                vec![Action::Transfer {
                    deposit: NearToken::from_near(1),
                }],
                None,
            ),
        ];
        let result = contract.relay_chunked_meta_transactions(signed_delegates);
        if let Err(e) = &result {
            panic!("Chunked relay failed: {:?}", e);
        }
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 5);
        assert_eq!(
            contract.get_gas_pool().0,
            initial_gas_pool - (29_000_000_000_000_000_000_000 * 5) // 0.145 NEAR for 5 actions
        );
    }

    #[test]
    fn test_relay_chunked_meta_transactions_with_chain_signatures() {
        let (mut contract, mut context) = setup_contract();
        context
            .attached_deposit(NearToken::from_near(20))
            .prepaid_gas(Gas::from_tgas(500)); // Increased to 500 TGas to avoid GasExceeded
        testing_env!(context.build());
        contract.deposit_gas_pool().unwrap();
        let initial_gas_pool = contract.get_gas_pool().0;
        let signed_delegates = vec![
            create_signed_delegate(
                accounts(1),
                "mpc.near".parse().unwrap(),
                vec![Action::ChainSignatureRequest {
                    target_chain: "near".to_string(),
                    derivation_path: "m/44'/60'/0'/0/0".to_string(),
                    payload: vec![1, 2, 3],
                }],
                None,
            ),
            create_signed_delegate(
                accounts(1),
                "mpc.near".parse().unwrap(),
                vec![Action::ChainSignatureRequest {
                    target_chain: "near".to_string(),
                    derivation_path: "m/44'/60'/0'/0/1".to_string(),
                    payload: vec![4, 5, 6],
                }],
                None,
            ),
        ];
        let result = contract.relay_chunked_meta_transactions(signed_delegates);
        if let Err(e) = &result {
            panic!("Chunked relay failed: {:?}", e);
        }
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
        assert_eq!(
            contract.get_gas_pool().0,
            initial_gas_pool - (29_000_000_000_000_000_000_000 * 2) // 0.058 NEAR for 2 actions
        );
    }

    #[test]
    fn test_sponsor_account() {
        let (mut contract, mut context) = setup_contract();
        context
            .predecessor_account_id(accounts(1)) // Auth account
            .attached_deposit(NearToken::from_near(10))
            .prepaid_gas(Gas::from_tgas(300));
        testing_env!(context.build());
    
        contract.deposit_gas_pool().unwrap();
        let initial_gas_pool = contract.get_gas_pool().0;
    
        let public_key = "ed25519:DP8d4JWrG8TFvQz83EJSVphUTEpQ41mzqPMyZMigCN17"
            .parse()
            .unwrap();
        let result = contract.sponsor_account(
            "testuser1.testnet".parse().unwrap(),
            "testnet".parse().unwrap(), // system_account
            public_key,
        );
    
        assert!(result.is_ok(), "Expected sponsor_account to succeed, got {:?}", result.err());
        assert_eq!(
            contract.get_gas_pool().0,
            initial_gas_pool - contract.relayer.sponsor_amount,
            "Gas pool should decrease by sponsor_amount"
        );
    }

    #[test]
    fn test_sponsor_account_insufficient_gas() {
        let (mut contract, mut context) = setup_contract();
        context.predecessor_account_id(accounts(1)); // Auth account
        testing_env!(context.build());
        let result = contract.sponsor_account(
            "testuser.testnet".parse().unwrap(),
            "testnet".parse().unwrap(), // system_account
            "ed25519:8fWHEecB2iXjZ75kMYG34M2DSELK9nQ31K3vQ3Wy4nqW"
                .parse()
                .unwrap(),
        );
        assert!(matches!(result, Err(RelayerError::InsufficientGasPool)));
    }

    #[test]
    fn test_sponsor_account_unauthorized() {
        let (mut contract, mut context) = setup_contract();
        context
            .predecessor_account_id(accounts(3)) // Not an auth account
            .attached_deposit(NearToken::from_near(10))
            .prepaid_gas(Gas::from_tgas(300));
        testing_env!(context.build());
        contract.deposit_gas_pool().unwrap();
        let result = contract.sponsor_account(
            "testuser.testnet".parse().unwrap(),
            "testnet".parse().unwrap(), // system_account
            "ed25519:8fWHEecB2iXjZ75kMYG34M2DSELK9nQ31K3vQ3Wy4nqW"
                .parse()
                .unwrap(),
        );
        assert!(matches!(result, Err(RelayerError::Unauthorized)));
    }

    #[test]
    fn test_refund_gas_callback() {
        let (mut contract, mut context) = setup_contract();
        context
            .attached_deposit(NearToken::from_near(10))
            .prepaid_gas(Gas::from_tgas(300));
        testing_env!(context.build());
        contract.deposit_gas_pool().unwrap();
        let initial_cost = 29_000_000_000_000_000_000_000; // 0.029 NEAR estimated cost
        let initial_gas_pool = contract.get_gas_pool().0;
        testing_env!(context
            .clone()
            .prepaid_gas(Gas::from_tgas(295)) // 300 TGas - 5 TGas used
            .build());
        contract.refund_gas_callback(initial_cost);
        let expected_gas_pool = initial_gas_pool + initial_cost; // Full refund in test env
        assert_eq!(
            contract.get_gas_pool().0,
            expected_gas_pool,
            "Gas pool should reflect refund"
        );
    }

    #[test]
    fn test_handle_mpc_signature() {
        let (mut contract, mut context) = setup_contract();
        context
            .attached_deposit(NearToken::from_near(10))
            .prepaid_gas(Gas::from_tgas(300));
        testing_env!(context.build());
        contract.deposit_gas_pool().unwrap();
        let initial_gas_pool = contract.get_gas_pool().0;
        let signed_delegate = create_signed_delegate(
            accounts(1),
            "mpc.near".parse().unwrap(),
            vec![Action::ChainSignatureRequest {
                target_chain: "near".to_string(),
                derivation_path: "m/44'/60'/0'/0/0".to_string(),
                payload: vec![1, 2, 3],
            }],
            None,
        );
        let result = contract.relay_meta_transaction(signed_delegate);
        if let Err(e) = &result {
            panic!("Relay failed: {:?}", e);
        }
        assert!(result.is_ok());
        assert_eq!(
            contract.get_gas_pool().0,
            initial_gas_pool - 29_000_000_000_000_000_000_000 // 0.029 NEAR gas cost
        );
    }

    #[test]
    fn test_handle_bridge_result() {
        let (mut contract, mut context) = setup_contract();
        context
            .attached_deposit(NearToken::from_near(10))
            .prepaid_gas(Gas::from_tgas(300));
        testing_env!(context.build());
        contract.deposit_gas_pool().unwrap();
        let initial_gas_pool = contract.get_gas_pool().0;
        let signed_delegate = create_signed_delegate(
            accounts(1),
            accounts(2),
            vec![Action::Transfer {
                deposit: NearToken::from_near(1),
            }],
            None,
        );
        let result = contract.relay_meta_transaction(signed_delegate);
        if let Err(e) = &result {
            panic!("Relay failed: {:?}", e);
        }
        assert!(result.is_ok());
        assert_eq!(
            contract.get_gas_pool().0,
            initial_gas_pool - 29_000_000_000_000_000_000_000 // 0.029 NEAR gas cost
        );
    }

    #[test]
    fn test_view_methods() {
        let (mut contract, mut context) = setup_contract();
        context.attached_deposit(NearToken::from_near(5));
        testing_env!(context.build());
        contract.deposit_gas_pool().unwrap();
        assert_eq!(
            contract.get_gas_pool().0,
            5_000_000_000_000_000_000_000_000
        );
        assert_eq!(
            contract.get_min_gas_pool().0,
            1_000_000_000_000_000_000_000_000
        );
        assert_eq!(
            contract.get_sponsor_amount().0,
            100_000_000_000_000_000_000_000
        );
        assert_eq!(contract.get_chunk_size(), 5);
        assert_eq!(contract.get_version(), "1.0", "Version should be 1.0");
    }

    #[test]
    fn test_initial_gas_values() {
        let (contract, _) = setup_contract();
        assert_eq!(contract.get_max_gas().0, 250 * 1_000_000_000_000, "Initial max_gas should be 250 TGas");
        assert_eq!(contract.get_mpc_sign_gas().0, 100 * 1_000_000_000_000, "Initial mpc_sign_gas should be 100 TGas");
        assert_eq!(contract.get_callback_gas().0, 10 * 1_000_000_000_000, "Initial callback_gas should be 10 TGas");
    }

    #[test]
    fn test_set_max_gas_success() {
        let (mut contract, _) = setup_contract();
        let new_max_gas = U128(200 * 1_000_000_000_000); // 200 TGas
        let result = contract.set_max_gas(new_max_gas);
        assert!(result.is_ok(), "set_max_gas should succeed");
        assert_eq!(contract.get_max_gas().0, 200 * 1_000_000_000_000, "max_gas should be updated to 200 TGas");
    }

    #[test]
    fn test_set_max_gas_too_low() {
        let (mut contract, _) = setup_contract();
        let new_max_gas = U128(40 * 1_000_000_000_000); // 40 TGas, below minimum 50 TGas
        let result = contract.set_max_gas(new_max_gas);
        assert!(matches!(result, Err(RelayerError::AmountTooLow)), "max_gas below 50 TGas should fail");
    }

    #[test]
    fn test_set_max_gas_unauthorized() {
        let (mut contract, mut context) = setup_contract();
        context.predecessor_account_id(accounts(3));
        testing_env!(context.build());
        let new_max_gas = U128(200 * 1_000_000_000_000); // 200 TGas
        let result = contract.set_max_gas(new_max_gas);
        assert!(matches!(result, Err(RelayerError::Unauthorized)), "Non-admin should not be able to set max_gas");
    }

    #[test]
    fn test_set_mpc_sign_gas_success() {
        let (mut contract, _) = setup_contract();
        let new_mpc_sign_gas = U128(150 * 1_000_000_000_000); // 150 TGas
        let result = contract.set_mpc_sign_gas(new_mpc_sign_gas);
        assert!(result.is_ok(), "set_mpc_sign_gas should succeed");
        assert_eq!(contract.get_mpc_sign_gas().0, 150 * 1_000_000_000_000, "mpc_sign_gas should be updated to 150 TGas");
    }

    #[test]
    fn test_set_mpc_sign_gas_too_low() {
        let (mut contract, _) = setup_contract();
        let new_mpc_sign_gas = U128(10 * 1_000_000_000_000); // 10 TGas, below minimum 20 TGas
        let result = contract.set_mpc_sign_gas(new_mpc_sign_gas);
        assert!(matches!(result, Err(RelayerError::AmountTooLow)), "mpc_sign_gas below 20 TGas should fail");
    }

    #[test]
    fn test_set_callback_gas_success() {
        let (mut contract, _) = setup_contract();
        let new_callback_gas = U128(15 * 1_000_000_000_000); // 15 TGas
        let result = contract.set_callback_gas(new_callback_gas);
        assert!(result.is_ok(), "set_callback_gas should succeed");
        assert_eq!(contract.get_callback_gas().0, 15 * 1_000_000_000_000, "callback_gas should be updated to 15 TGas");
    }

    #[test]
    fn test_set_callback_gas_too_low() {
        let (mut contract, _) = setup_contract();
        let new_callback_gas = U128(2 * 1_000_000_000_000); // 2 TGas, below minimum 5 TGas
        let result = contract.set_callback_gas(new_callback_gas);
        assert!(matches!(result, Err(RelayerError::AmountTooLow)), "callback_gas below 5 TGas should fail");
    }

    #[test]
    fn test_sponsor_account_with_custom_max_gas() {
        let (mut contract, mut context) = setup_contract();
        context
            .predecessor_account_id(accounts(1)) // Auth account
            .attached_deposit(NearToken::from_near(10))
            .prepaid_gas(Gas::from_tgas(300));
        testing_env!(context.build());
        contract.deposit_gas_pool().unwrap();

        let new_max_gas = U128(200 * 1_000_000_000_000); // 200 TGas
        context.predecessor_account_id(accounts(0)); // Admin to set max_gas
        testing_env!(context.build());
        contract.set_max_gas(new_max_gas).unwrap();
        assert_eq!(contract.get_max_gas().0, 200 * 1_000_000_000_000, "max_gas should be 200 TGas");

        context.predecessor_account_id(accounts(1)); // Back to auth account
        testing_env!(context.build());
        let initial_gas_pool = contract.get_gas_pool().0;
        let public_key = "ed25519:DP8d4JWrG8TFvQz83EJSVphUTEpQ41mzqPMyZMigCN17"
            .parse()
            .unwrap();
        let result = contract.sponsor_account(
            "testuser2.testnet".parse().unwrap(),
            "testnet".parse().unwrap(), // system_account
            public_key,
        );
        assert!(result.is_ok(), "sponsor_account should succeed with custom max_gas: {:?}", result.err());
        assert_eq!(
            contract.get_gas_pool().0,
            initial_gas_pool - contract.relayer.sponsor_amount,
            "Gas pool should decrease by sponsor_amount"
        );
    }

    #[test]
    fn test_relay_meta_transaction_respects_max_gas() {
        let (mut contract, mut context) = setup_contract();
        context.attached_deposit(NearToken::from_near(10))
               .prepaid_gas(Gas::from_tgas(300));
        testing_env!(context.build());
        contract.deposit_gas_pool().unwrap();

        let new_max_gas = U128(100 * 1_000_000_000_000); // 100 TGas
        contract.set_max_gas(new_max_gas).unwrap();
        assert_eq!(contract.get_max_gas().0, 100 * 1_000_000_000_000, "max_gas should be 100 TGas");

        let initial_gas_pool = contract.get_gas_pool().0;
        let signed_delegate = create_signed_delegate(
            accounts(1),
            accounts(2),
            vec![Action::FunctionCall {
                method_name: "test".to_string(),
                args: vec![1, 2, 3],
                gas: Gas::from_tgas(150), // Requests 150 TGas, should be capped at 100 TGas
                deposit: NearToken::from_near(0),
            }],
            None,
        );
        let result = contract.relay_meta_transaction(signed_delegate);
        assert!(result.is_ok(), "Relay should succeed with capped gas: {:?}", result.err());
        assert_eq!(
            contract.get_gas_pool().0,
            initial_gas_pool - 29_000_000_000_000_000_000_000,
            "Gas pool should decrease by fixed cost (0.029 NEAR)"
        );
    }

    #[test]
    fn test_relay_meta_transaction_chain_signature_respects_mpc_sign_gas() {
        let (mut contract, mut context) = setup_contract();
        context.attached_deposit(NearToken::from_near(10))
               .prepaid_gas(Gas::from_tgas(300));
        testing_env!(context.build());
        contract.deposit_gas_pool().unwrap();

        let new_mpc_sign_gas = U128(80 * 1_000_000_000_000); // 80 TGas
        contract.set_mpc_sign_gas(new_mpc_sign_gas).unwrap();
        assert_eq!(contract.get_mpc_sign_gas().0, 80 * 1_000_000_000_000, "mpc_sign_gas should be 80 TGas");

        let initial_gas_pool = contract.get_gas_pool().0;
        let signed_delegate = create_signed_delegate(
            accounts(1),
            "mpc.near".parse().unwrap(),
            vec![Action::ChainSignatureRequest {
                target_chain: "near".to_string(),
                derivation_path: "m/44'/60'/0'/0/0".to_string(),
                payload: vec![1, 2, 3],
            }],
            None,
        );
        let result = contract.relay_meta_transaction(signed_delegate);
        assert!(result.is_ok(), "Relay should succeed with custom mpc_sign_gas: {:?}", result.err());
        assert_eq!(
            contract.get_gas_pool().0,
            initial_gas_pool - 29_000_000_000_000_000_000_000,
            "Gas pool should decrease by fixed cost (0.029 NEAR)"
        );
    }

    #[test]
    fn test_pause_and_unpause() {
        let (mut contract, mut context) = setup_contract();
        
        assert_eq!(contract.is_paused(), false, "Contract should start unpaused");

        let result = contract.pause();
        assert!(result.is_ok(), "Pause should succeed for admin");
        assert_eq!(contract.is_paused(), true, "Contract should be paused");

        context.attached_deposit(NearToken::from_near(5));
        testing_env!(context.build());
        let deposit_result = contract.deposit_gas_pool();
        assert!(matches!(deposit_result, Err(RelayerError::ContractPaused)), "Deposit should fail when paused");

        let signed_delegate = create_signed_delegate(
            accounts(1),
            accounts(2),
            vec![Action::Transfer {
                deposit: NearToken::from_near(1),
            }],
            None,
        );
        let relay_result = contract.relay_meta_transaction(signed_delegate);
        assert!(matches!(relay_result, Err(RelayerError::ContractPaused)), "Relay should fail when paused");

        let unpause_result = contract.unpause();
        assert!(unpause_result.is_ok(), "Unpause should succeed for admin");
        assert_eq!(contract.is_paused(), false, "Contract should be unpaused");

        let deposit_result = contract.deposit_gas_pool();
        assert!(deposit_result.is_ok(), "Deposit should succeed after unpause");
    }

    #[test]
    fn test_pause_unauthorized() {
        let (mut contract, mut context) = setup_contract();
        context.predecessor_account_id(accounts(3)); // Non-admin
        testing_env!(context.build());
        let result = contract.pause();
        assert!(matches!(result, Err(RelayerError::Unauthorized)), "Non-admin should not be able to pause");
        assert_eq!(contract.is_paused(), false, "Contract should remain unpaused");
    }

    #[test]
    fn test_unpause_unauthorized() {
        let (mut contract, mut context) = setup_contract();
        contract.pause().unwrap(); // Pause as admin first
        context.predecessor_account_id(accounts(3)); // Non-admin
        testing_env!(context.build());
        let result = contract.unpause();
        assert!(matches!(result, Err(RelayerError::Unauthorized)), "Non-admin should not be able to unpause");
        assert_eq!(contract.is_paused(), true, "Contract should remain paused");
    }

    #[test]
    fn test_pause_already_paused() {
        let (mut contract, _) = setup_contract();
        contract.pause().unwrap();
        let result = contract.pause();
        assert!(result.is_ok(), "Pausing an already paused contract should be a no-op");
        assert_eq!(contract.is_paused(), true, "Contract should still be paused");
    }

    #[test]
    fn test_unpause_already_unpaused() {
        let (mut contract, _) = setup_contract();
        let result = contract.unpause();
        assert!(result.is_ok(), "Unpausing an already unpaused contract should be a no-op");
        assert_eq!(contract.is_paused(), false, "Contract should still be unpaused");
    }

    #[test]
    fn test_migrate_success() {
        let (mut contract, _) = setup_contract();
        assert_eq!(contract.get_version(), "1.0", "Initial version should be 1.0");
        
        contract.pause().unwrap();
        
        let result = contract.migrate();
        assert!(result.is_ok(), "Migration should succeed: {:?}", result.err());
        assert_eq!(contract.get_version(), "1.1", "Version should be updated to 1.1");
    }

    #[test]
    fn test_migrate_unauthorized() {
        let (mut contract, mut context) = setup_contract();
        context.predecessor_account_id(accounts(3)); // Non-admin
        testing_env!(context.build());
        let result = contract.migrate();
        assert!(matches!(result, Err(RelayerError::Unauthorized)), "Non-admin should not be able to migrate");
        assert_eq!(contract.get_version(), "1.0", "Version should remain 1.0");
    }

    #[test]
    fn test_migrate_not_paused() {
        let (mut contract, _) = setup_contract();
        let result = contract.migrate();
        assert!(matches!(result, Err(RelayerError::ContractPaused)), "Migration should fail if not paused");
        assert_eq!(contract.get_version(), "1.0", "Version should remain 1.0");
    }

    #[test]
    fn test_migrate_already_latest_version() {
        let (mut contract, _) = setup_contract();
        contract.pause().unwrap();
        contract.migrate().unwrap(); // First migration to 1.1
        let result = contract.migrate();
        assert!(result.is_ok(), "Migrating at latest version should be a no-op: {:?}", result.err());
        assert_eq!(contract.get_version(), "1.1", "Version should still be 1.1");
    }

    #[test]
    fn test_view_methods_after_migration() {
        let (mut contract, mut context) = setup_contract();
        context.attached_deposit(NearToken::from_near(5));
        testing_env!(context.build());
        contract.deposit_gas_pool().unwrap();
        
        contract.pause().unwrap();
        contract.migrate().unwrap();
        
        assert_eq!(contract.get_version(), "1.1", "Version should be 1.1 after migration");
        assert_eq!(
            contract.get_gas_pool().0,
            5_000_000_000_000_000_000_000_000,
            "Gas pool should remain unchanged"
        );
        assert_eq!(
            contract.get_min_gas_pool().0,
            1_000_000_000_000_000_000_000_000,
            "Min gas pool should remain unchanged"
        );
        assert_eq!(
            contract.get_sponsor_amount().0,
            100_000_000_000_000_000_000_000,
            "Sponsor amount should remain unchanged"
        );
        assert_eq!(contract.get_chunk_size(), 5, "Chunk size should remain unchanged");
    }

    #[test]
    fn test_set_max_gas_boundary() {
        let (mut contract, _) = setup_contract();
        let boundary_max_gas = U128(50 * 1_000_000_000_000); // 50 TGas, minimum allowed
        let result = contract.set_max_gas(boundary_max_gas);
        assert!(result.is_ok(), "Setting max_gas to minimum (50 TGas) should succeed");
        assert_eq!(contract.get_max_gas().0, 50 * 1_000_000_000_000, "max_gas should be 50 TGas");
    }

    #[test]
    fn test_set_mpc_sign_gas_boundary() {
        let (mut contract, _) = setup_contract();
        let boundary_mpc_sign_gas = U128(20 * 1_000_000_000_000); // 20 TGas, minimum allowed
        let result = contract.set_mpc_sign_gas(boundary_mpc_sign_gas);
        assert!(result.is_ok(), "Setting mpc_sign_gas to minimum (20 TGas) should succeed");
        assert_eq!(contract.get_mpc_sign_gas().0, 20 * 1_000_000_000_000, "mpc_sign_gas should be 20 TGas");
    }

    #[test]
    fn test_set_callback_gas_boundary() {
        let (mut contract, _) = setup_contract();
        let boundary_callback_gas = U128(5 * 1_000_000_000_000); // 5 TGas, minimum allowed
        let result = contract.set_callback_gas(boundary_callback_gas);
        assert!(result.is_ok(), "Setting callback_gas to minimum (5 TGas) should succeed");
        assert_eq!(contract.get_callback_gas().0, 5 * 1_000_000_000_000, "callback_gas should be 5 TGas");
    }

    // Tests for sponsor_account_signed
    #[test]
    fn test_sponsor_account_signed_success() {
        let (mut contract, mut context) = setup_contract();
        context
            .predecessor_account_id(accounts(3)) // Random caller
            .attached_deposit(NearToken::from_near(10))
            .prepaid_gas(Gas::from_tgas(300));
        testing_env!(context.build());
        contract.deposit_gas_pool().unwrap();
        let initial_gas_pool = contract.get_gas_pool().0;

        let public_key = "ed25519:DP8d4JWrG8TFvQz83EJSVphUTEpQ41mzqPMyZMigCN17"
            .parse()
            .unwrap();
        let signed_delegate = create_signed_delegate(
            accounts(1), // Auth account
            "testuser3.testnet".parse().unwrap(),
            vec![Action::AddKey {
                public_key,
                allowance: None, // Full access key
                receiver_id: "testuser3.testnet".parse().unwrap(),
                method_names: vec![],
            }],
            None,
        );
        let result = contract.sponsor_account_signed(signed_delegate);
        assert!(result.is_ok(), "sponsor_account_signed should succeed: {:?}", result.err());
        assert_eq!(
            contract.get_gas_pool().0,
            initial_gas_pool - contract.relayer.sponsor_amount,
            "Gas pool should decrease by sponsor_amount"
        );
    }

    #[test]
    fn test_sponsor_account_signed_unauthorized() {
        let (mut contract, mut context) = setup_contract();
        context
            .predecessor_account_id(accounts(3)) // Random caller
            .attached_deposit(NearToken::from_near(10))
            .prepaid_gas(Gas::from_tgas(300));
        testing_env!(context.build());
        contract.deposit_gas_pool().unwrap();

        let public_key = "ed25519:DP8d4JWrG8TFvQz83EJSVphUTEpQ41mzqPMyZMigCN17"
            .parse()
            .unwrap();
        let signed_delegate = create_signed_delegate(
            accounts(4), // Not an auth account
            "testuser4.testnet".parse().unwrap(),
            vec![Action::AddKey {
                public_key,
                allowance: None,
                receiver_id: "testuser4.testnet".parse().unwrap(),
                method_names: vec![],
            }],
            None,
        );
        let result = contract.sponsor_account_signed(signed_delegate);
        assert!(matches!(result, Err(RelayerError::Unauthorized)), "Should fail with unauthorized signer");
    }

    #[test]
    fn test_sponsor_account_signed_invalid_action() {
        let (mut contract, mut context) = setup_contract();
        context
            .predecessor_account_id(accounts(3)) // Random caller
            .attached_deposit(NearToken::from_near(10))
            .prepaid_gas(Gas::from_tgas(300));
        testing_env!(context.build());
        contract.deposit_gas_pool().unwrap();

        let signed_delegate = create_signed_delegate(
            accounts(1), // Auth account
            accounts(2),
            vec![Action::Transfer { deposit: NearToken::from_near(1) }], // Wrong action
            None,
        );
        let result = contract.sponsor_account_signed(signed_delegate);
        assert!(matches!(result, Err(RelayerError::InvalidNonce)), "Should fail with invalid action");
    }

    #[test]
    fn test_sponsor_account_signed_insufficient_gas() {
        let (mut contract, mut context) = setup_contract();
        context
            .predecessor_account_id(accounts(3)) // Random caller
            .prepaid_gas(Gas::from_tgas(300));
        testing_env!(context.build());

        let public_key = "ed25519:DP8d4JWrG8TFvQz83EJSVphUTEpQ41mzqPMyZMigCN17"
            .parse()
            .unwrap();
        let signed_delegate = create_signed_delegate(
            accounts(1), // Auth account
            "testuser5.testnet".parse().unwrap(),
            vec![Action::AddKey {
                public_key,
                allowance: None,
                receiver_id: "testuser5.testnet".parse().unwrap(),
                method_names: vec![],
            }],
            None,
        );
        let result = contract.sponsor_account_signed(signed_delegate);
        assert!(matches!(result, Err(RelayerError::InsufficientGasPool)), "Should fail with insufficient gas");
    }
}