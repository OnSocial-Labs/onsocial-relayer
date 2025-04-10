# OnSocialRelayer

`OnSocialRelayer` is a NEAR Protocol smart contract designed to facilitate meta-transactions, gas pool management, account sponsoring, and basic **chain abstraction** for cross-chain signature requests. It enables authorized users to relay transactions, sponsors new NEAR accounts, and supports chain-agnostic signature requests via Multi-Party Computation (MPC) contract integration.

## Features

- **Meta-Transaction Relaying**: Execute signed delegate actions on behalf of users (single, batch, or chunked).
- **Gas Pool Management**: Deposit NEAR to fund transactions, with excess offloaded to a recipient and refund callbacks.
- **Account Sponsoring**: Create new NEAR accounts with a configurable sponsor amount, via direct calls or signed actions.
- **Chain Abstraction**: Relay signature requests to a target chain’s MPC contract for cross-chain operations.
- **Admin Controls**: Manage authorized accounts, admins, gas limits, chain mappings, and contract state, restricted to admins.
- **Pause/Unpause**: Temporarily halt contract operations (e.g., deposits, relaying, sponsoring) with admin control.
- **Event Logging**: Emit NEP-297 events for key actions (e.g., auth changes, gas updates, signature results, pause/unpause, migration).
- **Migration Support**: Upgrade contract state (e.g., from v1.0 to v1.1) while paused, preserving functionality.

## Chain Abstraction

The contract supports chain abstraction via the `ChainSignatureRequest` action, allowing users to request signatures from an MPC contract on a specified chain. Key aspects:

1. Users submit a `SignedDelegateAction` with a `ChainSignatureRequest` containing `target_chain`, `derivation_path`, and `payload`.
2. The contract maps the `target_chain` to an MPC contract (e.g., `"near" → "mpc.near"`) and forwards the request with configurable gas (`mpc_sign_gas`, default 100 TGas).
3. Results are logged via the `CrossChainSignatureResult` event.

### Example Use Case

Request a signature for an Ethereum transaction:
- Action: `ChainSignatureRequest { target_chain: "ethereum", derivation_path: "m/44'/60'/0'/0/0", payload: [/* tx data */] }`
- Relayed to: Configurable MPC contract (e.g., `mpc.eth.near`).

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (with `cargo`)
- [NEAR CLI](https://docs.near.org/tools/near-cli#installation)
- NEAR account for deployment (e.g., via [NEAR Wallet](https://wallet.near.org/))
- Optional: Access to an MPC contract for chain abstraction testing

## Installation

1. **Clone the Repository**:
   ```bash
   git clone https://github.com/yourusername/onsocialrelayer.git
   cd onsocialrelayer

Install Dependencies:
bash

cargo build --release

Set Up NEAR Environment:
Log in to NEAR CLI:
bash

near login

Ensure you have a NEAR account and sufficient funds.

Building and Deploying
Build the Contract:
bash

cargo build --target wasm32-unknown-unknown --release

Output: target/wasm32-unknown-unknown/release/onsocialrelayer.wasm

Deploy to NEAR Testnet:
bash

near deploy --accountId your-account.near --wasmFile target/wasm32-unknown-unknown/release/onsocialrelayer.wasm

Initialize the Contract:
bash

near call your-account.near new '{"admins": ["admin.near"], "initial_auth_account": "user.near", "initial_auth_key": "ed25519:YOUR_PUBLIC_KEY", "offload_recipient": "recipient.near"}' --accountId your-account.near

Usage
Depositing to Gas Pool
bash

near call your-account.near deposit_gas_pool --accountId your-account.near --deposit 5

Relaying a Meta-Transaction
Basic transfer:
bash

near call your-account.near relay_meta_transaction '{"signed_delegate": {"delegate_action": {"sender_id": "user.near", "receiver_id": "target.near", "actions": [{"Transfer": {"deposit": "1000000000000000000000000"}}], "nonce": 1, "max_block_height": 1000}, "signature": "YOUR_SIGNATURE", "public_key": "ed25519:YOUR_PUBLIC_KEY", "session_nonce": 1, "scheme": "Ed25519", "fee_action": null}}' --accountId your-account.near

Relaying a Chain Signature Request
bash

near call your-account.near relay_meta_transaction '{"signed_delegate": {"delegate_action": {"sender_id": "user.near", "receiver_id": "mpc.near", "actions": [{"ChainSignatureRequest": {"target_chain": "ethereum", "derivation_path": "m/44'/60'/0'/0/0", "payload": [1, 2, 3]}}], "nonce": 1, "max_block_height": 1000}, "signature": "YOUR_SIGNATURE", "public_key": "ed25519:YOUR_PUBLIC_KEY", "session_nonce": 1, "scheme": "Ed25519", "fee_action": null}}' --accountId your-account.near

Sponsoring an Account
Direct call:
bash

near call your-account.near sponsor_account '{"new_account_id": "newuser.testnet", "system_account": "testnet", "public_key": "ed25519:NEW_PUBLIC_KEY"}' --accountId your-account.near

Signed delegate:
bash

near call your-account.near sponsor_account_signed '{"signed_delegate": {"delegate_action": {"sender_id": "user.near", "receiver_id": "newuser.testnet", "actions": [{"AddKey": {"public_key": "ed25519:NEW_PUBLIC_KEY", "allowance": null, "receiver_id": "newuser.testnet", "method_names": []}}], "nonce": 1, "max_block_height": 1000}, "signature": "YOUR_SIGNATURE", "public_key": "ed25519:YOUR_PUBLIC_KEY", "session_nonce": 1, "scheme": "Ed25519", "fee_action": null}}' --accountId your-account.near

Admin Operations
Add an authorized account:
bash

near call your-account.near add_auth_account '{"auth_account": "newuser.near", "auth_public_key": "ed25519:NEW_PUBLIC_KEY"}' --accountId admin.near

Set chunk size:
bash

near call your-account.near set_chunk_size '{"new_size": 10}' --accountId admin.near

Add chain mapping:
bash

near call your-account.near add_chain_mpc_mapping '{"chain": "ethereum", "mpc_contract": "mpc.eth.near"}' --accountId admin.near

Pause the contract:
bash

near call your-account.near pause --accountId admin.near

Unpause the contract:
bash

near call your-account.near unpause --accountId admin.near

Set max gas (e.g., 200 TGas):
bash

near call your-account.near set_max_gas '{"new_max": "200000000000000"}' --accountId admin.near

Migrate contract state (when paused):
bash

near call your-account.near migrate --accountId admin.near

Testing
Run tests:
bash

cargo test

Test Coverage
Admin functions (auth accounts, admins, settings, pause/unpause, migration)

Gas pool management (deposits, refunds, limits)

Relaying (single, batch, chunked; transfers, function calls, chain signatures)

Sponsoring accounts (direct and signed)

Gas configuration (max gas, MPC sign gas, callback gas)

View methods

Error cases (unauthorized, insufficient gas, invalid inputs, paused state)

Notes
Tests use mock signatures; real signature verification requires integration with an MPC contract.

Refund callback behavior may differ in unit tests vs. live network; test end-to-end on testnet.

Add tests for nonce/expiration validation and complex action combinations as needed.

Project Structure

onsocialrelayer/
├── Cargo.toml          # Dependencies and build config
├── src/
│   ├── lib.rs          # Main contract entry point
│   ├── admin.rs        # Admin functions (incl. pause/unpause, migration)
│   ├── errors.rs       # Custom error types
│   ├── events.rs       # NEP-297 event definitions
│   ├── gas_pool.rs     # Gas pool management
│   ├── relay.rs        # Meta-transaction relaying (includes chain abstraction)
│   ├── sponsor.rs      # Account sponsoring logic
│   ├── state.rs        # Contract state definition (includes versioning)
│   ├── types.rs        # Data structures (includes ChainSignatureRequest)
│   └── tests/          # Unit tests (in lib.rs under #[cfg(test)])

Configuration
Gas Pool Limits
min_gas_pool: 1 NEAR

max_gas_pool: 500 NEAR

sponsor_amount: 0.1 NEAR (configurable, min 0.01 NEAR)

Gas Settings
max_gas: 250 TGas (configurable, min 50 TGas)

mpc_sign_gas: 100 TGas (configurable, min 20 TGas)

callback_gas: 10 TGas (configurable, min 5 TGas)

Other Settings
chunk_size: 5 (default, configurable 1-100)

Chain Mapping: Default "near" → "mpc.near", configurable

Paused State: Initially unpaused, toggleable by admins

Version: Starts at "1.0", migratable to "1.1" (adds versioning support)

Contributing
Fork the repository.

Create a feature branch (git checkout -b feature/your-feature).

Commit changes (git commit -m "Add your feature").

Push to the branch (git push origin feature/your-feature).

Open a pull request.

License
MIT License. See LICENSE for details.

