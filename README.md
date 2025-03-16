# OnSocialRelayer

A NEAR smart contract designed to create and manage subaccounts and top-level accounts under the `onsocial` namespace, with dynamic sponsorship for Mainnet subaccounts.

## Overview

This contract facilitates account creation on the NEAR blockchain:

- **Subaccounts**: Creates subaccounts under `.onsocial.near` (Mainnet) with a sponsored balance of 0.05 NEAR, or `.onsocial.testnet` (Testnet) with a default balance of 0.001 NEAR.
- **Top-Level Accounts**: Creates standalone accounts (e.g., `alice.testnet`) with a default balance of 0.1 NEAR.
- **Gas**: Uses 100 TGas for callbacks to optimize transaction costs.
- **Tracking**: Logs account creation events (success or failure) using the NEP-297 standard.

## Prerequisites

- **Rust**: Install with `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh` and add the WASM target: `rustup target add wasm32-unknown-unknown`.
- **NEAR CLI**: Install with `npm install -g near-cli` and authenticate with `near login`.
- **Accounts**:
  - `relayer.onsocial.testnet` (Testnet) with >0.01 NEAR for testing.
  - `relayer.onsocial.near` (Mainnet) with >0.1 NEAR for sponsorship and storage.

## Building

Compile the contract to WebAssembly:

```bash
cargo build --target wasm32-unknown-unknown --release
```
