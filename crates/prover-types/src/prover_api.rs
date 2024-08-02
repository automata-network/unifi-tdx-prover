use jsonrpsee::{core::RpcResult, proc_macros::rpc};
use serde::{Deserialize, Serialize};

use crate::Pob;

#[rpc(server, client, namespace = "prover")]
pub trait ProverApi {
    #[method(name = "prove")]
    async fn prove(&self, req: ProveReq) -> RpcResult<ProveResp>;
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProveReq {
    pub pob: Option<Pob>,
    pub client: Option<ProveByExecutionNode>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProveByExecutionNode {
    pub url: String,
    pub start_block: u64,
    pub end_block: u64,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ProveResp {}
