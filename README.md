# OnSocialRelayer

`OnSocialRelayer` is a NEAR Protocol smart contract designed to facilitate meta-transactions, gas pool management, account sponsoring, and basic **chain abstraction** for cross-chain signature requests. It enables authorized users to relay transactions, sponsors new NEAR accounts, and supports chain-agnostic signature requests via an MPC (Multi-Party Computation) contract integration.

## Features

- **Meta-Transaction Relaying**: Execute signed delegate actions on behalf of users.
- **Gas Pool Management**: Deposit NEAR to fund transactions, with excess offloaded to a recipient.
- **Account Sponsoring**: Create new NEAR accounts with a predefined amount.
- **Chain Abstraction**: Relay signature requests to a target chain’s MPC contract (e.g., for cross-chain operations).
- **Admin Controls**: Manage authorized accounts and settings, restricted to admins.
- **Event Logging**: Emit NEP-297 events for key actions (e.g., adding/removing auth accounts).

## Chain Abstraction

The contract supports a basic form of chain abstraction through the `ChainSignatureRequest` action, allowing users to request signatures from an MPC contract on a specified target chain. This is useful for cross-chain interactions where NEAR acts as a relay hub. The process:

1. Users submit a `SignedDelegateAction` with a `ChainSignatureRequest` action.
2. The contract forwards the request to the specified MPC contract (e.g., `mpc.target-chain.near`) with a payload, derivation path, and gas allocation.
3. The MPC contract processes the signature, abstracting the target chain’s specifics from the user.

### Example Use Case

Request a signature for an Ethereum transaction:

- Action: `ChainSignatureRequest { target_chain: "eth|mpc.eth.near", derivation_path: "m/44'/60'/0'/0/0", payload: [/* tx data */] }`
- Relayed to: `mpc.eth.near` for signing.

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (with `cargo`)
- [NEAR CLI](https://docs.near.org/tools/near-cli#installation)
- NEAR account for deployment (e.g., via [NEAR Wallet](https://wallet.near.org/))
- Optional: Access to an MPC contract for chain abstraction testing.

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

Output: target/wasm32-unknown-unknown/release/onsocialrelayer.wasm.

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

near call your-account.near relay_meta_transaction '{"signed_delegate": {"delegate_action": {"sender_id": "user.near", "receiver_id": "target.near", "actions": [{"Transfer": {"deposit": "1000000000000000000000000"}}], "nonce": 1, "max_block_height": 1000}, "signature": "YOUR_SIGNATURE", "public_key": "ed25519:YOUR_PUBLIC_KEY", "session_nonce": 1}}' --accountId your-account.near

Relaying a Chain Signature Request
Request a signature for a target chain:
bash

near call your-account.near relay_meta_transaction '{"signed_delegate": {"delegate_action": {"sender_id": "user.near", "receiver_id": "mpc.target-chain.near", "actions": [{"ChainSignatureRequest": {"target_chain": "eth|mpc.eth.near", "derivation_path": "m/44'/60'/0'/0/0", "payload": [1, 2, 3]}}], "nonce": 1, "max_block_height": 1000}, "signature": "YOUR_SIGNATURE", "public_key": "ed25519:YOUR_PUBLIC_KEY", "session_nonce": 1}}' --accountId your-account.near

Sponsoring an Account
bash

near call your-account.near sponsor_account '{"account_name": "newuser", "public_key": "ed25519:NEW_PUBLIC_KEY"}' --accountId your-account.near

Admin Operations
Add an authorized account:
bash

near call your-account.near add_auth_account '{"auth_account": "newuser.near", "auth_public_key": "ed25519:NEW_PUBLIC_KEY"}' --accountId admin.near

Testing
Run Tests:
bash

cargo test

Test Coverage:
Admin functions, gas pool, relaying, sponsoring, and view methods.

Chain abstraction: Limited testing for ChainSignatureRequest (no signature verification yet).

Notes:
Signature verification is not implemented; tests use dummy signatures.

Add tests for ChainSignatureRequest with a mock MPC contract for full coverage.

Project Structure

onsocialrelayer/
├── Cargo.toml          # Dependencies and build config
├── src/
│   ├── lib.rs          # Main contract entry point
│   ├── admin.rs        # Admin functions
│   ├── errors.rs       # Custom error types
│   ├── events.rs       # NEP-297 event definitions
│   ├── gas_pool.rs     # Gas pool management
│   ├── relay.rs        # Meta-transaction relaying (includes chain abstraction)
│   ├── sponsor.rs      # Account sponsoring logic
│   ├── state.rs        # Contract state definition
│   ├── types.rs        # Data structures (includes ChainSignatureRequest)
│   └── tests/          # Unit tests

Configuration
Gas Pool Limits:
min_gas_pool: 1 NEAR

max_gas_pool: 500 NEAR

Sponsor Amount: 0.1 NEAR

Max Gas per Action: 300 TGas

Chain Abstraction: Configurable via target_chain in ChainSignatureRequest.

Contributing
Fork the repository.

Create a feature branch (git checkout -b feature/your-feature).

Commit changes (git commit -m "Add your feature").

Push to the branch (git push origin feature/your-feature).

Open a pull request.

License
MIT License. See LICENSE for details.

