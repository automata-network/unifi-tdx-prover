use jsonrpsee::{core::RpcResult, proc_macros::rpc};
use serde::{Deserialize, Serialize};

use crate::{Pob, H256};

#[rpc(server, client, namespace = "prover")]
pub trait ProverApi {
    #[method(name = "prove")]
    async fn prove(&self, req: ProveReq) -> RpcResult<ProveResp>;
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProveReq {
    // `pob` and `client`, at least one must be provided
    // if both are provided, `pob` will be used.

    // execute the blocks based on the Pob
    pub pob: Option<Pob>,
    // generate the pob based on the execution node
    // and then executes the blocks
    pub client: Option<ProveByExecutionNode>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProveByExecutionNode {
    pub url: String,
    pub start_block: u64,
    pub end_block: u64,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ProveResp {
    pub new_state_root: H256,
}
