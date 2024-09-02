use std::sync::Arc;

use alloy_sol_types::SolValue;
use base::{stack_error, Keypair};
use executor::{BlockExecutor, ExecutionError};
use jsonrpsee::{
    core::{async_trait, RpcResult},
    types::ErrorObject,
};
use reth_primitives::U256;

use crate::{Pob, Poe, ProofInput, ProofRequest, ProofResponse, ProverV1ApiServer};

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

pub fn prove(input: ProofInput, kp: &Keypair, tee_type: U256) -> Result<ProofResponse, ProveError> {
    let pob: Arc<Pob> = Arc::new(input.into());
    let new_block = BlockExecutor::new(pob.clone()).execute()?;
    let version = 1u64;
    let poe = Poe {
        state_root: new_block.header.state_root,
        parent_hash: pob.data.l2_parent_header.hash_slow(),
        block_hash: new_block.hash_slow(),
        graffiti: pob.data.graffiti,
    };

    let (id, addr, sk) = kp.info().ok_or(ProveError::ProverNotRegistered)?;

    let poe = poe.sign(&pob, id, addr, &sk, tee_type);
    log::info!("poe: {:?}", poe);
    Ok(ProofResponse {
        version,
        data: poe.abi_encode().into(),
    })
}

pub struct Prover {
    tee_type: U256,
    kp: Keypair,
}

impl Prover {
    pub fn new(kp: Keypair, tee_type: U256) -> Self {
        Self { kp, tee_type }
    }
}

#[async_trait]
impl ProverV1ApiServer for Prover {
    async fn gen_proof(&self, req: ProofRequest) -> RpcResult<ProofResponse> {
        let response = prove(req.input, &self.kp, self.tee_type)
            .map_err(|err| ErrorObject::owned(14001, format!("{:?}", err), None::<()>))?;
        Ok(response)
    }
}
