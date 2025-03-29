# OnSocial Relayer Contract

A NEAR smart contract designed for meta-transactions, account sponsoring, and gas management within the OnSocial ecosystem.

## Overview

The OnSocial Relayer contract enables gasless transactions, account creation sponsorship, and gas pool management on the NEAR blockchain. It supports meta-transactions to whitelisted contracts, sponsors new accounts (named or implicit), and provides admin controls for configuration. Built with `near-sdk`, it includes robust error handling, NEP-297 event logging, and a retry mechanism for failed transactions.

## Features

- **Meta-Transaction Relaying**: Executes signed transactions on behalf of users to whitelisted contracts.
- **Account Sponsoring**: Funds creation of named (e.g., `user.testnet`) or implicit (64-character hex) accounts.
- **Gas Pool Management**: Funds operations via a NEAR deposit pool.
- **Admin Controls**: Configurable whitelist, gas limits, sponsor amounts, and admin list.
- **Failed Transaction Queue**: Queues and retries failed transactions (up to 100) with increased gas (120% + buffer, capped at 300 TGas).
- **Event Emission**: Logs actions using NEP-297 events for transparency.
- **Testing Utilities**: Simulation methods for signature and promise results in test mode.

## Prerequisites

- **NEAR Wallet**: Use [wallet.testnet.near.org](https://wallet.testnet.near.org) for Testnet or [wallet.near.org](https://wallet.near.org) for Mainnet.
- **NEAR CLI**: Install via `npm install -g near-cli`.
- **Rust Toolchain**: Install with `rustup install stable` (tested with Rust 1.74+ as of March 2025) and add WASM target via `rustup target add wasm32-unknown-unknown`.
- **cargo-near**: Install with `cargo install cargo-near`.
- **Dependencies**: Requires `ed25519-dalek` for signature verification (included in `Cargo.toml`).
- **Basic NEAR Knowledge**: Familiarity with accounts, keys, and transactions.

## Installation

1. **Clone the Repository**:
   ```bash
   git clone https://github.com/yourusername/onsocialrelayer.git
   cd onsocialrelayer

Install Dependencies:
bash

rustup target add wasm32-unknown-unknown
cargo install cargo-near

Build the Contract:
bash

cargo build --target wasm32-unknown-unknown --release

Contract Methods
Initialization
new(payment_ft_contract: Option<AccountId>, min_ft_payment: U128, whitelisted_contracts: Vec<AccountId>)
Initializes the contract with defaults: 0.1 NEAR sponsor amount, 150 TGas default gas, 50 TGas buffer, 300-block max height delta (~5-10 minutes).

Args: Optional FT payment contract, minimum FT payment (yoctoNEAR), initial whitelist.

Defaults: Adds social.near, social.tkn.near, USDC Testnet (3e2210e...), USDC Mainnet (1720862...), and admins (onsocial.sputnik-dao.near, onsocial.testnet, onsocial.near).

Public Methods
deposit_gas_pool() (Payable)
Deposits NEAR into the gas pool.

Requires: Attached deposit > 0.

Event: GasPoolDeposited { amount, depositor }.

on_receive_near() (Payable)
Handles incoming NEAR transfers to the gas pool.

Requires: Amount > 0.

Event: GasPoolDeposited { amount, depositor }.

get_gas_pool() -> U128
Returns the current gas pool balance in yoctoNEAR.

get_sponsor_amount() -> U128
Returns the sponsor amount in yoctoNEAR (default: 0.1 NEAR).

get_admins() -> Vec<AccountId>
Returns the list of admin account IDs.

get_default_gas() -> Gas
Returns the default gas limit (default: 150 TGas).

get_gas_buffer() -> Gas
Returns the gas buffer (default: 50 TGas).

get_failed_transactions_count() -> u32
Returns the number of queued failed transactions.

get_failed_transactions() -> Vec<(SignedDelegateAction, u64, Option<RelayerError>)>
Returns the list of failed transactions with gas and error details.

get_failed_transactions_by_sender(sender_id: AccountId) -> Vec<(SignedDelegateAction, u64, Option<RelayerError>)>
Returns failed transactions for a specific sender.

get_processed_nonce(account_id: AccountId) -> Option<u64>
Returns the last processed nonce for an account.

get_max_block_height_delta() -> u64
Returns the default max block height delta (default: 300 blocks).

get_pending_transaction(sender_id: AccountId, nonce: u64) -> Option<(u64, bool)>
Checks the status of a pending transaction (max block height, expired).

sponsor_account(account_name: String, public_key: PublicKey, add_function_call_key: bool, is_implicit: bool, signed_delegate: Option<SignedDelegateAction>) -> Result<Promise, RelayerError>
Sponsors a new account via direct call (self-only) or signed delegate action.

Args: Account name, public key, function call key flag, implicit flag, optional delegate.

Requires: Sufficient gas pool, unique account ID, admin or valid signature.

Events: AccountSponsored, FunctionCallKeyAdded (if key added).

relay_meta_transaction(signed_delegate: SignedDelegateAction) -> Result<(), RelayerError>
Relays a signed meta-transaction.

Args: Signed delegate action (sender, receiver, actions, nonce, max block height, signature, public key).

Requires: Valid nonce, Ed25519 signature, whitelisted receiver, sufficient gas.

Event: MetaTransactionRelayed.

import_account(account_id: AccountId, public_key: PublicKey, signed_delegate: SignedDelegateAction) -> Result<Promise, RelayerError>
Imports an existing account by adding a function call key.

Requires: Valid signature, sufficient gas pool.

Event: FunctionCallKeyAdded.

Admin Methods
update_whitelist(contracts: Vec<AccountId>) -> Result<(), RelayerError>
Updates the whitelist of allowed receiver contracts.

Requires: Admin caller.

set_sponsor_amount(amount: U128) -> Result<(), RelayerError>
Sets the sponsor amount (minimum 0.05 NEAR).

Requires: Admin caller.

set_admins(new_admins: Vec<AccountId>) -> Result<(), RelayerError>
Updates the admin list (non-empty).

Requires: Admin caller.

add_function_call_key(account_id: AccountId, public_key: PublicKey, receiver_id: AccountId, method_names: Vec<String>) -> Result<(), RelayerError>
Adds a function call key to an account.

Requires: Admin caller.

Event: FunctionCallKeyAdded.

remove_function_call_key(account_id: AccountId, public_key: PublicKey, signed_delegate: SignedDelegateAction) -> Result<Promise, RelayerError>
Removes a function call key from an account.

Requires: Valid signature, sufficient gas.

Event: FunctionCallKeyRemoved.

set_gas_config(default_gas_tgas: u64, gas_buffer_tgas: u64) -> Result<(), RelayerError>
Sets gas limits (minimum 50 TGas default, 10 TGas buffer).

Requires: Admin caller.

set_max_block_height_delta(delta: u64) -> Result<(), RelayerError>
Sets the max block height delta (100-10,000 blocks).

Requires: Admin caller.

retry_or_clear_failed_transactions(retry: bool) -> Result<(), RelayerError>
Retries (120% gas + buffer, capped at 300 TGas) or clears failed transactions.

Args: true to retry, false to clear.

Requires: Admin caller.

Events: FailedTransactionsRetried, FailedTransactionsCleared.

Test-Only Methods
set_simulate_signature_failure(fail: bool)
Simulates signature verification failure for testing.

set_simulate_promise_result(result: Option<SerializablePromiseResult>)
Simulates promise outcomes for testing.

Events (NEP-297)
MetaTransactionRelayed { sender_id: AccountId, nonce: u64 }

AccountSponsored { account_id: AccountId, public_key: PublicKey, is_implicit: bool }

GasPoolDeposited { amount: NearToken, depositor: AccountId }

FunctionCallKeyAdded { account_id: AccountId, public_key: PublicKey, receiver_id: AccountId }

FunctionCallKeyRemoved { account_id: AccountId, public_key: PublicKey }

FailedTransactionsCleared { count: u32 }

FailedTransactionsRetried { count: u32 }

View Events:
bash

near view $CONTRACT_ID --logs

Error Types (RelayerError)
InsufficientGasPool: Gas pool below 1 NEAR.

InvalidNonce: Nonce reused or out of sequence.

NotWhitelisted: Receiver not in whitelist.

InvalidSignature: Ed25519 signature verification failed.

NoActions: Delegate action has no operations.

InvalidFTTransfer: FT transfer not to relayer or wrong method/contract.

InsufficientDeposit: FT payment below minimum.

InsufficientBalance: Insufficient funds for sponsoring.

AccountExists: Account already sponsored.

Unauthorized: Caller not an admin.

InvalidSponsorAmount: Amount below 0.05 NEAR.

InvalidKeyAction: Invalid key action parameters.

InvalidAccountId: Malformed account name or implicit ID.

ExpiredTransaction: Block height exceeds max_block_height.

InvalidGasConfig: Gas settings below minimums.

NoFailedTransactions: No transactions to retry/clear.

Dependencies
Defined in Cargo.toml:
near-sdk = "5.11.0": NEAR smart contract framework.

serde = { version = "1.0", features = ["derive"] }: Serialization/deserialization.

serde_json = "1.0": JSON handling.

borsh = { version = "1.5.7", features = ["unstable__schema"] }: Borsh serialization.

ed25519-dalek = "2.1.1": Ed25519 signature verification.

Environment Variables
Set for convenience:
bash

export CONTRACT_ID="onsocialrelayer.testnet"
export ACCOUNT_ID="youraccount.testnet"
export ADMIN_ID="onsocial.testnet"

Usage Examples
Deposit to Gas Pool
bash

near call $CONTRACT_ID deposit_gas_pool --accountId $ACCOUNT_ID --amount 5

Sponsor an Account
bash

near call $CONTRACT_ID sponsor_account '{"account_name": "user123", "public_key": "ed25519:abc123...", "add_function_call_key": false, "is_implicit": false}' --accountId $CONTRACT_ID

Note: Direct calls are restricted to the contract itself; use a signed delegate action for external calls.
Relay a Meta-Transaction
json

{
  "signed_delegate": {
    "delegate_action": {
      "sender_id": "user.testnet",
      "receiver_id": "social.near",
      "actions": [
        {
          "Transfer": {
            "deposit": "1000000000000000000000000"
          }
        }
      ],
      "nonce": 1,
      "max_block_height": 1000000
    },
    "signature": "ed25519:<64-byte-hex-signature>",
    "public_key": "ed25519:xyz789..."
  }
}

bash

near call $CONTRACT_ID relay_meta_transaction '<json-above>' --accountId $ACCOUNT_ID

Note: Generate the signature using the sender’s private key over the Borsh-serialized DelegateAction.
Check Gas Pool
bash

near view $CONTRACT_ID get_gas_pool

Deployment Instructions
Build:
bash

cargo build --target wasm32-unknown-unknown --release

Deploy to Testnet:
bash

cargo near deploy --account-id $ACCOUNT_ID --wasm-file target/wasm32-unknown-unknown/release/onsocialrelayer.wasm

Initialize:
bash

near call $CONTRACT_ID new '{"payment_ft_contract": null, "min_ft_payment": "0", "whitelisted_contracts": ["social.near"]}' --accountId $ACCOUNT_ID

Verify:
bash

near view $CONTRACT_ID get_admins

Testing
Run unit tests:
bash

cargo test

Tests cover initialization, gas pool deposits, account sponsoring, meta-transaction relaying, admin functions, and edge cases.

Uses near-sdk’s testing framework with simulated environments.

Contributing
Bugs: Report via GitHub issues.

Features: Suggest via issues or submit pull requests (PRs).

Code: Fork the repo, create a feature branch, add tests, and submit a PR.

License
MIT License.
Troubleshooting
Insufficient Gas Pool: Deposit more NEAR with deposit_gas_pool.

Invalid Signature: Verify the signature matches the sender’s public key and serialized DelegateAction.

Not Whitelisted: Request an admin to add the contract via update_whitelist.

Transaction Expired: Increase max_block_height in the meta-transaction.

Queued Failed Transactions: Check with get_failed_transactions_count and use retry_or_clear_failed_transactions.

