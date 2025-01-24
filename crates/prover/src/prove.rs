use alloy_primitives::Address;
use alloy_sol_types::SolValue;
use base::{stack_error, Keypair};
use executor::{BlockExecutor, ExecutionError};
use jsonrpsee::{
    core::{async_trait, RpcResult},
    types::ErrorObject,
};
use raiko_core::{
    interfaces::ProofRequest as RpcProofRequest, provider::rpc::RpcBlockDataProvider, Raiko,
};
use raiko_lib::consts::SupportedChainSpecs;
use raiko_lib::input::GuestInput;
use reth_primitives::U256;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{
    MultiProofRequest, Pob, Poe, ProofInput, ProofRequest, ProofResponse, ProverV1ApiServer,
    SignedPoe,
};

stack_error! {
    name: ProveError,
    stack_name: ProveErrorStack,
    error: {
        ProverNotRegistered,
        MissingPoe,
        BlockHashMismatch{ idx: usize, cur: Poe, prev: Poe },
    },
    wrap: {
        Execution(ExecutionError),
        Json(serde_json::Error),
    },
    stack: {
        SerdePoe(),
        SignPoe(),
        BlockNumber(block_num: u64),
    }
}

pub fn prove(
    input: ProofInput,
    prover_registry: Address,
    kp: &Keypair,
    tee_type: U256,
) -> Result<SignedPoe, ProveError> {
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

pub async fn prove_multi_blocks(
    inputs: Vec<ProofInput>,
    worker_num: usize,
    prover_registry: Address,
    kp: &Keypair,
    tee_type: U256,
) -> Result<SignedPoe, ProveError> {
    let pobs = inputs
        .iter()
        .map(|n| n.clone().into())
        .collect::<Vec<Pob>>();
    let poes = base::parallel((), inputs, worker_num, |input, _| async move {
        let pob: Arc<Pob> = Arc::new(input.into());
        match BlockExecutor::new(pob.clone()).execute() {
            Ok(new_block) => Ok(Poe {
                state_root: new_block.header.state_root,
                parent_hash: pob.data.l2_parent_header.hash_slow(),
                block_hash: new_block.hash_slow(),
                graffiti: pob.data.graffiti,
            }),
            Err(err) => return Err(err).map_err(ProveError::BlockNumber(&pob.block.number)),
        }
    })
    .await?;

    let (id, addr, sk) = kp.info().ok_or(ProveError::ProverNotRegistered)?;
    let poe = Poe::sign_multi(&poes, &pobs, id, prover_registry, addr, &sk, tee_type)
        .map_err(ProveError::SignPoe())?;
    Ok(poe)
}

pub struct Prover {
    tee_type: U256,
    prover_registry: Address,
    kp: Keypair,
    worker_num: usize,
}

impl Prover {
    pub fn new(kp: Keypair, prover_registry: Address, tee_type: U256, worker_num: usize) -> Self {
        Self {
            kp,
            prover_registry,
            tee_type,
            worker_num,
        }
    }

    pub async fn get_proof(&self, req: RpcProofRequest) -> GuestInput {
        let path = "./chain_spec_list.json".parse().unwrap();
        let chain_specs = SupportedChainSpecs::merge_from_file(path).unwrap();

        let taiko_chain_spec = chain_specs.get_chain_spec(&req.network).unwrap();
        let l1_chain_spec = chain_specs.get_chain_spec(&req.l1_network).unwrap();

        let provider = RpcBlockDataProvider::new(&taiko_chain_spec.rpc, req.block_number - 1)
            .expect("Could not create RpcBlockDataProvider");

        let raiko = Raiko::new(l1_chain_spec, taiko_chain_spec, req.clone());

        raiko
            .generate_input(provider)
            .await
            .expect("input generation failed")
    }

    pub async fn get_proofs(
        &self,
        RpcMultiProofRequest {
            request,
            start_block,
            end_block,
        }: RpcMultiProofRequest,
    ) -> RpcResult<Vec<GuestInput>> {
        let path = "./chain_spec_list.json".parse().unwrap();
        let chain_specs = SupportedChainSpecs::merge_from_file(path).unwrap();

        let taiko_chain_spec = chain_specs
            .get_chain_spec(&request.network)
            .ok_or_else(|| {
                ErrorObject::owned(
                    14001,
                    format!("failed to get chain spec: {:?}", request.network),
                    None::<()>,
                )
            })?;
        let l1_chain_spec = chain_specs
            .get_chain_spec(&request.l1_network)
            .ok_or_else(|| {
                ErrorObject::owned(
                    14001,
                    format!("failed to get chain spec: {:?}", request.l1_network),
                    None::<()>,
                )
            })?;

        let mut reqs = Vec::with_capacity((end_block - start_block) as usize);
        for blk_num in start_block..=end_block {
            let mut tmp_req = request.clone();
            tmp_req.block_number = blk_num;
            reqs.push(tmp_req);
        }

        let proofs = base::parallel((), reqs, self.worker_num, move |req, _| {
            let l1_chain_spec = l1_chain_spec.clone();
            let taiko_chain_spec = taiko_chain_spec.clone();
            async move {
                let provider =
                    RpcBlockDataProvider::new(&taiko_chain_spec.rpc, req.block_number - 1)
                        .map_err(|err| {
                            ErrorObject::owned(
                                14001,
                                format!("Could not create RpcBlockDataProvider: {:?}", err),
                                None::<()>,
                            )
                        })?;
                let raiko = Raiko::new(l1_chain_spec.clone(), taiko_chain_spec.clone(), req);
                let proof = raiko.generate_input(provider).await.map_err(|err| {
                    ErrorObject::owned(
                        14001,
                        format!("Could not generate input: {:?}", err),
                        None::<()>,
                    )
                })?;
                Ok::<GuestInput, ErrorObject>(proof)
            }
        })
        .await?;

        Ok(proofs)
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

    pub async fn prove_multi(&self, req: MultiProofRequest) -> RpcResult<ProofResponse> {
        let version = 1u64;
        let signed_poe =
            prove_multi_blocks(req.input, 4, self.prover_registry, &self.kp, self.tee_type)
                .await
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RpcMultiProofRequest {
    pub request: RpcProofRequest,
    pub start_block: u64,
    pub end_block: u64,
}
