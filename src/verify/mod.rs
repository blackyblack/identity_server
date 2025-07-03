use ethers_core::{rand, types::H160};
use ethers_signers::{LocalWallet, Signer};

use crate::{identity::UserAddress, verify::error::Error};

pub mod admin;
pub mod error;
pub mod forget;
pub mod moderator;
pub mod nonce;
pub mod nonce_db;
pub mod proof;
pub mod punish;
pub mod signature;
pub mod vouch;

pub fn address_to_string(address: &H160) -> String {
    format!("0x{}", hex::encode(address.as_bytes()))
}

// useful for tests and examples
pub fn random_keypair() -> (String, UserAddress) {
    let wallet = LocalWallet::new(&mut rand::thread_rng());
    let private_key = hex::encode(wallet.signer().to_bytes());
    let address = address_to_string(&wallet.address());
    (private_key, address)
}

pub fn private_key_to_wallet(private_key_hex: &str) -> Result<LocalWallet, Error> {
    let private_key_hex = private_key_hex.trim_start_matches("0x");
    let private_key_bytes = hex::decode(private_key_hex)?;
    LocalWallet::from_bytes(&private_key_bytes)
        .map_err(|e| Error::WalletCreationError(format!("Failed to create wallet: {:?}", e)))
}

pub fn private_key_to_address(private_key_hex: &str) -> Result<UserAddress, Error> {
    let wallet = private_key_to_wallet(private_key_hex)?;
    Ok(address_to_string(&wallet.address()))
}
