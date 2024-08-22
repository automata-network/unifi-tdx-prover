use std::sync::Arc;

use base::stack_error;
use executor::{BlockExecutor, ExecutionError};
use jsonrpsee::{
    core::{async_trait, RpcResult},
    types::ErrorObject,
};
use reth_primitives::U256;
use secp256k1::SecretKey;

use crate::{Pob, Poe, ProofInput, ProofRequest, ProofResponse, ProverV1ApiServer};

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

pub fn prove(input: ProofInput, sk: &SecretKey) -> Result<ProofResponse, ProveError> {
    let pob: Arc<Pob> = Arc::new(input.into());
    let new_block = BlockExecutor::new(pob.clone()).execute()?;
    let version = 1u64;
    let poe = Poe {
        version: U256::from_limbs_slice(&[version]),
        prev_state_root: pob.data.prev_state_root,
        new_state_root: new_block.header.state_root,
    };
    let poe = poe.sign(sk);
    let bytes = serde_json::to_vec(&poe).map_err(ProveError::SerdePoe())?;
    Ok(ProofResponse {
        version,
        data: bytes.into(),
    })
}

pub struct Prover {
    sk: SecretKey,
}

impl Prover {
    pub fn new(sk: SecretKey) -> Self {
        Self { sk }
    }
}

#[async_trait]
impl ProverV1ApiServer for Prover {
    async fn gen_proof(&self, req: ProofRequest) -> RpcResult<ProofResponse> {
        let response = prove(req.input, &self.sk)
            .map_err(|err| ErrorObject::owned(14001, format!("{:?}", err), None::<()>))?;
        Ok(response)
    }
}
