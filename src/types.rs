use near_sdk::{AccountId, Gas, NearToken, PublicKey};
use near_sdk::borsh::{BorshSerialize, BorshDeserialize};
use serde::{Deserialize, Serialize};

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct DelegateAction {
    pub sender_id: AccountId,
    pub receiver_id: AccountId,
    pub actions: Vec<Action>,
    pub nonce: u64,
    pub max_block_height: u64,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct SignedDelegateAction {
    pub delegate_action: DelegateAction,
    pub signature: Vec<u8>,
    pub public_key: PublicKey,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub enum Action {
    FunctionCall { method_name: String, args: Vec<u8>, gas: Gas, deposit: NearToken },
    Transfer { deposit: NearToken },
    AddKey { public_key: PublicKey, allowance: Option<NearToken>, receiver_id: AccountId, method_names: Vec<String> },
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub enum SerializablePromiseResult {
    Successful(Vec<u8>),
    Failed,
}

impl From<near_sdk::PromiseResult> for SerializablePromiseResult {
    fn from(result: near_sdk::PromiseResult) -> Self {
        match result {
            near_sdk::PromiseResult::Successful(data) => Self::Successful(data),
            near_sdk::PromiseResult::Failed => Self::Failed,
            #[cfg(not(test))]
            near_sdk::PromiseResult::NotReady => Self::Failed,
        }
    }
}