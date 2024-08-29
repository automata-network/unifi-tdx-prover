use std::sync::{Arc, Mutex};

use raiko_lib::primitives::keccak::keccak;
use reth_primitives::Address;
use secp256k1::{rand::thread_rng, Message, PublicKey, SecretKey, SECP256K1};

#[derive(Clone, Debug)]
pub struct Keypair {
    key: Arc<Mutex<(Arc<SecretKey>, Arc<PublicKey>)>>,
}

impl Keypair {
    pub fn new() -> Self {
        let (sk, pk) = secp256k1::generate_keypair(&mut thread_rng());
        Self {
            key: Arc::new(Mutex::new((Arc::new(sk), Arc::new(pk)))),
        }
    }

    pub fn address(&self) -> Address {
        let hash = keccak(&self.public_key().serialize_uncompressed()[1..]);
        Address::from_slice(&hash[12..])
    }

    pub fn secret_key(&self) -> Arc<SecretKey> {
        self.key.lock().unwrap().0.clone()
    }

    pub fn public_key(&self) -> Arc<PublicKey> {
        self.key.lock().unwrap().1.clone()
    }

    pub fn rotate(&self) -> KeypairRotate {
        KeypairRotate {
            kp: Keypair::new(),
            old_key: self,
        }
    }

    pub fn sign_digest_ecdsa(&self, digest: [u8; 32]) -> [u8; 65] {
        let msg = Message::from_digest(digest);
        let sig = SECP256K1.sign_ecdsa_recoverable(&msg, &self.secret_key());
        let (v, rs) = sig.serialize_compact();
        let mut sig = [0_u8; 65];
        sig[..64].copy_from_slice(&rs[..]);
        sig[64] = v.to_i32() as u8;
        sig
    }
}

pub struct KeypairRotate<'a> {
    kp: Keypair,
    old_key: &'a Keypair,
}

impl<'a> KeypairRotate<'a> {
    pub fn commit(self) {
        *self.old_key.key.lock().unwrap() = self.kp.key.lock().unwrap().clone()
    }
}

impl<'a> std::ops::Deref for KeypairRotate<'a> {
    type Target = Keypair;
    fn deref(&self) -> &Self::Target {
        &self.kp
    }
}
