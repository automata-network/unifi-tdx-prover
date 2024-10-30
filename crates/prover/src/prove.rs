use std::sync::Arc;
use alloy_primitives::Address;
use alloy_sol_types::SolValue;
use base::{stack_error, Keypair};
use executor::{BlockExecutor, ExecutionError};
use jsonrpsee::{
    core::{async_trait, RpcResult},
    types::ErrorObject,
};
use raiko_core::{
    interfaces::{
        ProofRequest as RpcProofRequest,
    },
    provider::{
        rpc::{
            RpcBlockDataProvider,
        }
    },
    Raiko,
};
use raiko_lib::{
    consts::{
        SupportedChainSpecs
    }
};
use raiko_lib::input::GuestInput;
use reth_primitives::U256;

use crate::{Pob, Poe, ProofInput, ProofRequest, ProofResponse, ProverV1ApiServer, SignedPoe};

stack_error! {
    name: ProveError,
    stack_name: ProveErrorStack,
    error: {
        ProverNotRegistered,
    },
    wrap: {
        Execution(ExecutionError),
        Json(serde_json::Error),
    },
    stack: {
        SerdePoe(),
    }
}

pub fn prove(input: ProofInput, prover_registry: Address, kp: &Keypair, tee_type: U256) -> Result<SignedPoe, ProveError> {
    let pob: Arc<Pob> = Arc::new(input.into());
    let new_block = BlockExecutor::new(pob.clone()).execute()?;
    let poe = Poe {
        state_root: new_block.header.state_root,
        parent_hash: pob.data.l2_parent_header.hash_slow(),
        block_hash: new_block.hash_slow(),
        graffiti: pob.data.graffiti,
    };

    let (id, addr, sk) = kp.info().ok_or(ProveError::ProverNotRegistered)?;

    let poe = poe.sign(&pob, id, prover_registry, addr, &sk, tee_type);
    log::info!("poe: {:?}", poe);
    Ok(poe)
}

pub struct Prover {
    tee_type: U256,
    prover_registry: Address,
    kp: Keypair,
}

impl Prover {
    pub fn new(kp: Keypair, prover_registry: Address, tee_type: U256) -> Self {
        Self { kp, prover_registry, tee_type }
    }

    pub async fn get_proof(&self, req: RpcProofRequest) -> GuestInput {
        let l1_network = "holesky".to_owned();
        let network = "unifi_testnet".to_owned();
        let path = "./chain_spec_list.json".parse().unwrap();
        let chain_specs = SupportedChainSpecs::merge_from_file(path).unwrap();

        let taiko_chain_spec = chain_specs.get_chain_spec(&network).unwrap();
        let l1_chain_spec = chain_specs.get_chain_spec(&l1_network).unwrap();

        let provider =
            RpcBlockDataProvider::new(&taiko_chain_spec.rpc, req.block_number - 1)
                .expect("Could not create RpcBlockDataProvider");

        let raiko = Raiko::new(l1_chain_spec, taiko_chain_spec, req.clone());

        raiko
            .generate_input(provider)
            .await
            .expect("input generation failed")
    }

    pub fn prove(&self, req: ProofRequest) -> RpcResult<ProofResponse> {
        let version = 1u64;
        let signed_poe = prove(req.input, self.prover_registry, &self.kp, self.tee_type)
            .map_err(|err| ErrorObject::owned(14001, format!("{:?}", err), None::<()>))?;

        let id_be_bytes: [u8; 32] = signed_poe.id.to_be_bytes::<32>();
        let id: [u8; 4] = id_be_bytes[28..].try_into().unwrap();
        let new_instance: [u8; 20] = signed_poe.new_instance.into_array();
        let signature: [u8; 65] = signed_poe.signature.to_vec().try_into().unwrap();

        let data: [u8; 89] = [&id[..], &new_instance[..], &signature[..]]
            .concat()
            .try_into()
            .unwrap();

        Ok(ProofResponse {
            version,
            data: data.into(),
        })
    }
}

#[async_trait]
impl ProverV1ApiServer for Prover {
    async fn gen_proof(&self, req: ProofRequest) -> RpcResult<ProofResponse> {
        let version = 1u64;

        let response = prove(req.input, self.prover_registry, &self.kp, self.tee_type)
            .map_err(|err| ErrorObject::owned(14001, format!("{:?}", err), None::<()>))?;
        Ok(ProofResponse {
            version,
            data: response.abi_encode().into(),
        })
    }
}
