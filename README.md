# OnSocial Relayer Contract

A NEAR smart contract designed for meta-transactions, account sponsoring, and gas management within the OnSocial ecosystem.

## Overview

The OnSocial Relayer contract enables gasless transactions, account creation sponsorship, and gas pool management. It supports meta-transactions to whitelisted contracts, sponsors new accounts (named or implicit), and provides admin controls for configuration. It includes robust error handling, event logging via NEP-297, and a retry mechanism for failed transactions.

## Features

- **Meta-Transaction Relaying**: Executes signed transactions on behalf of users to whitelisted contracts.
- **Account Sponsoring**: Funds creation of named (e.g., `user.testnet`) or implicit accounts.
- **Gas Pool Management**: Funds operations via a NEAR deposit pool.
- **Admin Controls**: Configurable whitelist, gas limits, sponsor amounts, and admin list.
- **Failed Transaction Queue**: Queues and retries failed transactions (up to 100) with increased gas (120% + buffer, capped at 300 TGas).
- **Event Emission**: Logs actions using NEP-297 events.
- **Testing Utilities**: Simulation methods for signature and promise results.

## Prerequisites

- **NEAR Wallet**: Use [wallet.testnet.near.org](https://wallet.testnet.near.org) for Testnet or [wallet.near.org](https://wallet.near.org) for Mainnet.
- **NEAR CLI**: Install via `cargo install cargo-near`.
- **Rust Toolchain**: Install with `rustup install stable` (tested with Rust 1.74+ as of March 2025) and add WASM target via `rustup target add wasm32-unknown-unknown`.
- **Dependencies**: Requires `ed25519-dalek` for signature verification.
- **Basic NEAR Knowledge**: Familiarity with accounts, keys, and transactions.

## Contract Methods

### Initialization
- **`new(payment_ft_contract: Option<AccountId>, min_ft_payment: U128, whitelisted_contracts: Vec<AccountId>)`**
  - Initializes the contract with defaults: 100 yoctoNEAR sponsor amount, 150 TGas default gas, 50 TGas buffer.
  - **Args**: Optional FT payment contract, minimum FT payment (yoctoNEAR), initial whitelist.
  - **Defaults**: Adds `social.near`, `social.tkn.near`, USDC Testnet (`3e2210e1184b45b64c8a434c0a7e7b23cc04ea7eb7a6c3c32520d03d4afcb8af`), USDC Mainnet (`17208628f84f5d6ad33f0da3bbbeb27ffcb398eac501a31bd6ad2011e36133a1`), and admins (`onsocial.sputnik-dao.near`, `onsocial.testnet`, `onsocial.near`).

### Public Methods
- **`deposit_gas_pool()`** *(Payable)*
  - Deposits NEAR into the gas pool.
  - **Requires**: Attached deposit > 0.
  - **Event**: `GasPoolDeposited { amount, depositor }`.

- **`on_receive_near()`** *(Payable)*
  - Handles incoming NEAR transfers to the gas pool.
  - **Requires**: Amount > 0.
  - **Event**: `GasPoolDeposited { amount, depositor }`.

- **`get_gas_pool() -> U128`**
  - Returns the current gas pool balance in yoctoNEAR.

- **`get_sponsor_amount() -> U128`**
  - Returns the amount used for sponsoring accounts in yoctoNEAR.

- **`get_admins() -> Vec<AccountId>`**
  - Returns the list of admin account IDs.

- **`get_default_gas() -> Gas`**
  - Returns the default gas limit in Gas (default: 150 TGas).

- **`get_gas_buffer() -> Gas`**
  - Returns the gas buffer in Gas (default: 50 TGas).

- **`get_failed_transactions_count() -> u32`**
  - Returns the number of queued failed transactions.

- **`sponsor_account(account_name: String, public_key: PublicKey, add_function_call_key: bool, is_implicit: bool) -> Result<Promise, RelayerError>`**
  - Sponsors a new account.
  - **Args**: Account name, public key, optional function call key, implicit flag.
  - **Requires**: Sufficient gas pool balance, unique account ID.
  - **Events**: `AccountSponsored { account_id, public_key, is_implicit }`, `FunctionCallKeyAdded` (if key added).

- **`relay_meta_transaction(signed_delegate: SignedDelegateAction) -> Result<(), RelayerError>`**
  - Relays a signed meta-transaction. In test mode, signature verification can be simulated (see `set_simulate_signature_failure`).
  - **Args**: Signed delegate action with sender, receiver, actions, nonce, max block height, signature, and public key.
  - **Requires**: Valid nonce, ed25519 signature, whitelisted receiver, sufficient gas.
  - **Event**: `MetaTransactionRelayed { sender_id, nonce }`.

### Callback Methods
- **`callback_success(sender_id: AccountId, nonce: u64)`**
  - Updates nonce after a successful transaction.
  - **Event**: `MetaTransactionRelayed { sender_id, nonce }`.

- **`callback_failure(signed_delegate: SignedDelegateAction, gas: Gas)`**
  - Queues a failed transaction for retry (up to 100) with 120% gas + buffer, capped at 300 TGas.

- **`callback_key_addition(sender_id: AccountId)`**
  - Confirms function call key addition.
  - **Event**: `FunctionCallKeyAdded { account_id, public_key, receiver_id }`.

### Admin Methods
- **`update_whitelist(contracts: Vec<AccountId>) -> Result<(), RelayerError>`**
  - Updates the whitelist of allowed receiver contracts.
  - **Requires**: Caller must be an admin.

- **`set_sponsor_amount(amount: U128) -> Result<(), RelayerError>`**
  - Sets the sponsor amount (minimum 0.05 NEAR).
  - **Requires**: Caller must be an admin.

- **`set_admins(new_admins: Vec<AccountId>) -> Result<(), RelayerError>`**
  - Updates the admin list (must be non-empty).
  - **Requires**: Caller must be an admin.

- **`add_function_call_key(account_id: AccountId, public_key: PublicKey, receiver_id: AccountId, method_names: Vec<String>) -> Result<(), RelayerError>`**
  - Adds a function call key to an account.
  - **Requires**: Caller must be an admin.

- **`set_gas_config(default_gas_tgas: u64, gas_buffer_tgas: u64) -> Result<(), RelayerError>`**
  - Sets default gas and buffer in TGas (converted to Gas internally). Minimum 50 TGas default, 10 TGas buffer.
  - **Requires**: Caller must be an admin.

- **`retry_or_clear_failed_transactions(retry: bool) -> Result<(), RelayerError>`**
  - Retries (with 120% gas + buffer, capped at 300 TGas) or clears failed transactions.
  - **Args**: `true` to retry, `false` to clear.
  - **Requires**: Caller must be an admin.
  - **Events**: `FailedTransactionsRetried { count }`, `FailedTransactionsCleared { count }`.

### Test-Only Methods
- **`set_simulate_signature_failure(fail: bool)`**
  - Simulates signature verification failure for testing.

- **`set_simulate_promise_result(result: Option<SerializablePromiseResult>)`**
  - Simulates promise outcomes (`Successful` or `Failed`) for testing.

## Events (NEP-297)
- **`MetaTransactionRelayed { sender_id: AccountId, nonce: u64 }`**
- **`AccountSponsored { account_id: AccountId, public_key: PublicKey, is_implicit: bool }`**
- **`GasPoolDeposited { amount: NearToken, depositor: AccountId }`**
- **`FunctionCallKeyAdded { account_id: AccountId, public_key: PublicKey, receiver_id: AccountId }`**
- **`FailedTransactionsCleared { count: u32 }`**
- **`FailedTransactionsRetried { count: u32 }`**

**Example**: To view events:
```bash
near view $CONTRACT_ID --logs

Error Types
RelayerError:
InsufficientGasPool: Gas pool below minimum (1 NEAR).

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

InvalidKeyAction: Invalid key addition parameters.

InvalidAccountId: Malformed account name or implicit ID.

ExpiredTransaction: Block height exceeds max_block_height.

InvalidGasConfig: Gas settings below minimums.

NoFailedTransactions: No transactions to retry/clear.

Dependencies
near-sdk = "5.11.0": NEAR smart contract framework.

serde = "1.0": Serialization/deserialization.

serde_json = "1.0": JSON handling.

borsh = "1.5.7": Borsh serialization with schema support.

ed25519-dalek = "2.1.1": Ed25519 signature verification.

Environment Variables
Set these for convenience:
bash

export CONTRACT_ID="onsocialrelayer.testnet"
export ACCOUNT_ID="youraccount.testnet"
export ADMIN_ID="onsocial.testnet"

Common Use Cases
Sponsoring a New User: Create an account for an OnSocial user without requiring them to hold NEAR.

Relaying a Payment: Execute NEAR or token transfers on behalf of users, covering gas costs.

Funding Operations: Maintain the gas pool to support sponsoring and relaying.

Usage Examples
Deposit to Gas Pool
bash

near call $CONTRACT_ID deposit_gas_pool --accountId $ACCOUNT_ID --amount 5

Sponsor an Account
bash

near call $CONTRACT_ID sponsor_account '{"account_name": "user123", "public_key": "ed25519:abc123...", "add_function_call_key": false, "is_implicit": false}' --accountId $ADMIN_ID

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

Note: Generate the signature using the sender’s private key over the serialized DelegateAction (Borsh format).
Deployment Instructions
Install Dependencies
bash

rustup target add wasm32-unknown-unknown
cargo install cargo-near

Build
bash

cargo build --target wasm32-unknown-unknown --release

Deploy to Testnet
bash

cargo near deploy --account-id $ACCOUNT_ID --wasm-file target/wasm32-unknown-unknown/release/onsocialrelayer.wasm

Initialize
bash

near call $CONTRACT_ID new '{"payment_ft_contract": null, "min_ft_payment": "0", "whitelisted_contracts": ["social.near"]}' --accountId $ACCOUNT_ID

Verify
bash

near view $CONTRACT_ID get_admins

Contributing
Bugs: Report via GitHub issues.

Features: Suggest via issues or submit pull requests (PRs).

Code: Fork the repo, create a feature branch, add tests, and submit a PR.

License
MIT License.
Troubleshooting
Insufficient Gas Pool: Use deposit_gas_pool to add more NEAR.

Invalid Signature: Ensure the signature matches the sender’s public key and the serialized DelegateAction.

Not Whitelisted: Request an admin to add the target contract with update_whitelist.

Transaction Expired: Increase max_block_height in the meta-transaction.

Queued Failed Transactions: Check count with get_failed_transactions_count and retry/clear via retry_or_clear_failed_transactions.

