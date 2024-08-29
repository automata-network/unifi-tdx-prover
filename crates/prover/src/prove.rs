use std::sync::Arc;

use base::stack_error;
use executor::{BlockExecutor, ExecutionError};
use jsonrpsee::{
    core::{async_trait, RpcResult},
    types::ErrorObject,
};
use reth_primitives::U256;

use crate::{Keypair, Pob, Poe, ProofInput, ProofRequest, ProofResponse, ProverV1ApiServer};

stack_error! {
    name: ProveError,
    stack_name: ProveErrorStack,
    error: {

    },
    wrap: {
        Execution(ExecutionError),
        Json(serde_json::Error),
    },
    stack: {
        SerdePoe(),
    }
}

pub fn prove(input: ProofInput, kp: &Keypair) -> Result<ProofResponse, ProveError> {
    let pob: Arc<Pob> = Arc::new(input.into());
    let new_block = BlockExecutor::new(pob.clone()).execute()?;
    let version = 1u64;
    let poe = Poe {
        state_root: new_block.header.state_root,
        parent_hash: pob.data.l2_parent_header.hash_slow(),
        block_hash: new_block.hash_slow(),
        graffiti: pob.data.graffiti,
    };
    let id = U256::default();
    let poe = poe.sign(&pob, id, kp.address(), &kp);
    let bytes = serde_json::to_vec(&poe).map_err(ProveError::SerdePoe())?;
    Ok(ProofResponse {
        version,
        data: bytes.into(),
    })
}

pub struct Prover {
    kp: Keypair,
}

impl Prover {
    pub fn new(kp: Keypair) -> Self {
        Self { kp }
    }
}

#[async_trait]
impl ProverV1ApiServer for Prover {
    async fn gen_proof(&self, req: ProofRequest) -> RpcResult<ProofResponse> {
        let response = prove(req.input, &self.kp)
            .map_err(|err| ErrorObject::owned(14001, format!("{:?}", err), None::<()>))?;
        Ok(response)
    }
}
