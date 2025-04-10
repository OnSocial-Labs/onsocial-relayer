use near_sdk::borsh::{self, BorshSerialize};
use near_sdk::{AccountId, PublicKey as SdkPublicKey, NearToken};
use near_crypto::{SecretKey, Signature};
use base64::{Engine as _, engine::general_purpose};
use std::str::FromStr;
use clap::{Arg, Command};

#[derive(BorshSerialize)]
pub enum SignatureScheme { Ed25519 }
#[derive(BorshSerialize)]
pub enum Action { Transfer { deposit: NearToken } }
#[derive(BorshSerialize)]
pub struct DelegateAction {
    pub sender_id: AccountId,
    pub receiver_id: AccountId,
    pub actions: Vec<Action>,
    pub nonce: u64,
    pub max_block_height: u64,
}
#[derive(BorshSerialize)]
pub struct SignedDelegateAction {
    pub delegate_action: DelegateAction,
    pub signature: Vec<u8>,
    pub public_key: SdkPublicKey,
    pub session_nonce: u64,
    pub scheme: SignatureScheme,
    pub fee_action: Option<Action>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("serialize-borsh")
        .about("Generates a Borsh-serialized SignedDelegateAction for OnSocialRelayer testing")
        .arg(Arg::new("sender_id").long("sender_id").required(true).help("Sender account ID"))
        .arg(Arg::new("receiver_id").long("receiver_id").required(true).help("Receiver account ID"))
        .arg(Arg::new("nonce").long("nonce").required(true).value_parser(clap::value_parser!(u64)))
        .arg(Arg::new("max_block_height").long("max_block_height").required(true).value_parser(clap::value_parser!(u64)))
        .arg(Arg::new("private_key").long("private_key").required(true).help("Private key in ed25519:<base58> format"))
        .get_matches();

    let sender_id = AccountId::try_from(matches.get_one::<String>("sender_id").unwrap().to_string())?;
    let receiver_id = AccountId::try_from(matches.get_one::<String>("receiver_id").unwrap().to_string())?;
    let nonce = *matches.get_one::<u64>("nonce").unwrap();
    let max_block_height = *matches.get_one::<u64>("max_block_height").unwrap();
    let private_key_str = matches.get_one::<String>("private_key").unwrap();

    let secret_key = SecretKey::from_str(private_key_str)?;
    let public_key = secret_key.public_key();
    let sdk_public_key = SdkPublicKey::from_str(&public_key.to_string())?;

    let action = Action::Transfer { deposit: NearToken::from_yoctonear(1_000_000_000_000_000_000_000_000) }; // 1 NEAR
    let delegate_action = DelegateAction { sender_id, receiver_id, actions: vec![action], nonce, max_block_height };
    let serialized_delegate = borsh::to_vec(&delegate_action)?;
    let signature = secret_key.sign(&serialized_delegate);
    let signature_bytes = match signature {
        Signature::ED25519(sig) => sig.to_bytes().to_vec(),
        _ => unreachable!(),
    };

    let signed_delegate = SignedDelegateAction {
        delegate_action,
        signature: signature_bytes,
        public_key: sdk_public_key,
        session_nonce: 0,
        scheme: SignatureScheme::Ed25519,
        fee_action: None,
    };

    let serialized = borsh::to_vec(&signed_delegate)?;
    let base64_serialized = general_purpose::STANDARD.encode(&serialized);
    println!("Serialized (base64): {}", base64_serialized);
    println!("Public key: {}", public_key);
    Ok(())
}