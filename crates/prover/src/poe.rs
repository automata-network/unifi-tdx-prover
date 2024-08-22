use alloy_sol_types::SolValue;
use raiko_lib::primitives::keccak::keccak;
use secp256k1::{Message, SecretKey, SECP256K1};
use serde::{Deserialize, Serialize};

alloy_sol_types::sol! {
    #[derive(Default, Debug, Deserialize, Serialize)]
    struct Poe {
        uint256 version;
        bytes32 prev_state_root;
        bytes32 new_state_root;
    }

    #[derive(Default, Debug, Deserialize, Serialize)]
    struct SignedPoe {
        Poe poe;
        bytes signature;
    }
}

impl Poe {
    pub fn sign(self, sk: &SecretKey) -> SignedPoe {
        let msg = Message::from_digest(keccak(self.abi_encode()));
        let sig = SECP256K1.sign_ecdsa_recoverable(&msg, sk);
        let (v, rs) = sig.serialize_compact();
        let mut sig = [0_u8; 65];
        sig[..64].copy_from_slice(&rs[..]);
        sig[64] = v.to_i32() as u8;

        SignedPoe {
            poe: self,
            signature: sig.into(),
        }
    }
}
