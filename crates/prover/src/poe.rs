use alloy_sol_types::SolValue;
use executor::BlockDataProvider;
use raiko_lib::primitives::keccak::keccak;
use reth_primitives::{Address, U256};
use serde::{Deserialize, Serialize};

use crate::{Keypair, Pob};

alloy_sol_types::sol! {
    #[derive(Default, Debug, Deserialize, Serialize)]
    struct Poe {
        bytes32 parent_hash;
        bytes32 block_hash;
        bytes32 state_root;
        bytes32 graffiti;
    }

    #[derive(Default, Debug, Deserialize, Serialize)]
    struct SignedPoe {
        Poe poe;
        uint256 id;
        address new_instance;
        bytes signature;
    }
}

impl Poe {
    pub fn signed_msg(&self, pob: &Pob, new_instance: Address) -> Vec<u8> {
        (
            "VERIFY_PROOF",
            pob.chain_id(),
            pob.data.l1_contract.unwrap_or_default(),
            self.clone(),
            new_instance,
            pob.data.prover,
            pob.data.parent_meta_hash,
        )
            .abi_encode()
    }

    pub fn sign(self, pob: &Pob, id: U256, new_instance: Address, old_kp: &Keypair) -> SignedPoe {
        let sig = old_kp.sign_digest_ecdsa(keccak(self.signed_msg(pob, new_instance)));

        SignedPoe {
            poe: self,
            id,
            new_instance,
            signature: sig.into(),
        }
    }
}
