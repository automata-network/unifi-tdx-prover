use alloy_sol_types::SolValue;
use base::{Keypair, SecretKey};
use executor::BlockDataProvider;
use raiko_lib::primitives::keccak::keccak;
use reth_primitives::{hex, Address, Bytes, U256};
use serde::{Deserialize, Serialize};

use crate::{meta_hash, Pob};

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
        uint256 teeType; // 1: IntelTDX
    }
}

fn _check() {
    let _: base::ProverRegistryStub::Poe = unsafe { std::mem::transmute(Poe::default()) };
    let _: base::ProverRegistryStub::SignedPoe =
        unsafe { std::mem::transmute(SignedPoe::default()) };
    ()
}

impl Poe {
    pub fn signed_msg(&self, pob: &Pob, prover_registry: Address, new_instance: Address) -> Bytes {
        let mut vec = (
            "VERIFY_PROOF",
            pob.chain_id(),
            prover_registry,
            self.clone(),
            new_instance,
            pob.data.prover,
            meta_hash(&pob.data.block_meta),
        )
            .abi_encode();
        vec = (&vec[32..]).into();
        vec.into()
    }

    pub fn sign(
        self,
        pob: &Pob,
        id: U256,
        prover_registry: Address,
        new_instance: Address,
        sk: &SecretKey,
        tee_type: U256,
    ) -> SignedPoe {
        let sig = Keypair::sign_digest_ecdsa(sk, keccak(self.signed_msg(pob, prover_registry, new_instance)));

        SignedPoe {
            poe: self,
            id,
            new_instance,
            teeType: tee_type,
            signature: sig.into(),
        }
    }
}
