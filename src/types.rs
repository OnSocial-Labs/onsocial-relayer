use near_sdk::{AccountId, Gas, NearToken, PublicKey};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize, BorshSchema as NearBorshSchema};
use borsh::{BorshSchema as BorshSchemaTrait};
use borsh::schema::{Declaration, Definition};
use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, PartialEq, Eq, Hash, Ord, PartialOrd, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct WrappedAccountId(pub AccountId);

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct WrappedPublicKey(pub PublicKey);

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct WrappedGas(pub Gas);

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct WrappedNearToken(pub NearToken);

impl BorshSchemaTrait for WrappedAccountId {
    fn declaration() -> Declaration {
        "WrappedAccountId".to_string()
    }

    fn add_definitions_recursively(definitions: &mut BTreeMap<String, Definition>) {
        let def = Definition::Sequence {
            elements: "String".to_string(),
            length_width: 4,              // 4 bytes (u32) for length prefix
            length_range: 0..=u32::MAX as u64, // Range up to max u32 value
        };
        definitions.insert(Self::declaration(), def);
    }
}

impl BorshSchemaTrait for WrappedPublicKey {
    fn declaration() -> Declaration {
        "WrappedPublicKey".to_string()
    }

    fn add_definitions_recursively(definitions: &mut BTreeMap<String, Definition>) {
        let def = Definition::Sequence {
            elements: "u8".to_string(),
            length_width: 4,              // 4 bytes (u32) for length prefix
            length_range: 0..=u32::MAX as u64, // Range up to max u32 value
        };
        definitions.insert(Self::declaration(), def);
    }
}

impl BorshSchemaTrait for WrappedGas {
    fn declaration() -> Declaration {
        "WrappedGas".to_string()
    }

    fn add_definitions_recursively(definitions: &mut BTreeMap<String, Definition>) {
        let def = Definition::Primitive(8); // u64 is 8 bytes
        definitions.insert(Self::declaration(), def);
    }
}

impl BorshSchemaTrait for WrappedNearToken {
    fn declaration() -> Declaration {
        "WrappedNearToken".to_string()
    }

    fn add_definitions_recursively(definitions: &mut BTreeMap<String, Definition>) {
        let def = Definition::Primitive(16); // u128 is 16 bytes
        definitions.insert(Self::declaration(), def);
    }
}

impl From<AccountId> for WrappedAccountId {
    fn from(account_id: AccountId) -> Self {
        Self(account_id)
    }
}

impl From<WrappedAccountId> for AccountId {
    fn from(wrapper: WrappedAccountId) -> Self {
        wrapper.0
    }
}

impl From<PublicKey> for WrappedPublicKey {
    fn from(public_key: PublicKey) -> Self {
        Self(public_key)
    }
}

impl From<WrappedPublicKey> for PublicKey {
    fn from(wrapper: WrappedPublicKey) -> Self {
        wrapper.0
    }
}

impl From<Gas> for WrappedGas {
    fn from(gas: Gas) -> Self {
        Self(gas)
    }
}

impl From<WrappedGas> for Gas {
    fn from(wrapper: WrappedGas) -> Self {
        wrapper.0
    }
}

impl From<NearToken> for WrappedNearToken {
    fn from(near_token: NearToken) -> Self {
        Self(near_token)
    }
}

impl From<WrappedNearToken> for NearToken {
    fn from(wrapper: WrappedNearToken) -> Self {
        wrapper.0
    }
}

#[derive(BorshSerialize, BorshDeserialize, NearBorshSchema, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct DelegateAction {
    pub sender_id: WrappedAccountId,
    pub receiver_id: WrappedAccountId,
    pub actions: Vec<Action>,
    pub nonce: u64,
    pub max_block_height: u64,
}

#[derive(BorshSerialize, BorshDeserialize, NearBorshSchema, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct SignedDelegateAction {
    pub delegate_action: DelegateAction,
    pub signature: Vec<u8>,
    pub public_key: WrappedPublicKey,
}

#[derive(BorshSerialize, BorshDeserialize, NearBorshSchema, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub enum Action {
    FunctionCall { 
        method_name: String, 
        args: Vec<u8>, 
        gas: WrappedGas, 
        deposit: WrappedNearToken 
    },
    Transfer { 
        deposit: WrappedNearToken 
    },
    AddKey { 
        public_key: WrappedPublicKey, 
        allowance: Option<WrappedNearToken>, 
        receiver_id: WrappedAccountId, 
        method_names: Vec<String> 
    },
}

#[derive(BorshSerialize, BorshDeserialize, NearBorshSchema, Serialize, Deserialize, Clone)]
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
        }
    }
}