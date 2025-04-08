use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::{AccountId, Gas, NearToken, PublicKey};
use serde::{Serialize, Deserialize};
use near_sdk_macros::NearSchema;

#[derive(Clone, Eq, PartialEq, BorshSerialize, BorshDeserialize, Serialize, Deserialize, NearSchema)]
#[abi(borsh, json)]
pub enum Action {
    ChainSignatureRequest {
        target_chain: String,
        derivation_path: String,
        payload: Vec<u8>,
    },
    FunctionCall {
        method_name: String,
        args: Vec<u8>,
        gas: Gas, // Original field, capped at 290 TGas in relay.rs
        deposit: NearToken,
    },
    Transfer {
        deposit: NearToken,
    },
    AddKey {
        public_key: PublicKey,
        allowance: Option<NearToken>,
        receiver_id: AccountId,
        method_names: Vec<String>,
    },
}

#[derive(Clone, Eq, PartialEq, BorshSerialize, BorshDeserialize, Serialize, Deserialize, NearSchema)]
#[abi(borsh, json)]
pub struct DelegateAction {
    pub sender_id: AccountId,
    pub receiver_id: AccountId,
    pub actions: Vec<Action>,
    pub nonce: u64,
    pub max_block_height: u64,
}

#[derive(Clone, Eq, PartialEq, BorshSerialize, BorshDeserialize, Serialize, Deserialize, NearSchema)]
#[abi(borsh, json)]
pub enum SignatureScheme {
    Ed25519,
    // Add more schemes here in the future (e.g., Secp256k1)
}

#[derive(Clone, Eq, PartialEq, BorshSerialize, BorshDeserialize, Serialize, Deserialize, NearSchema)]
#[abi(borsh, json)]
pub struct SignedDelegateAction {
    pub delegate_action: DelegateAction,
    pub signature: Vec<u8>,
    pub public_key: PublicKey,
    pub session_nonce: u64,
    pub scheme: SignatureScheme,
    pub fee_action: Option<Action>, // NEW: Optional action for token-based fees
}