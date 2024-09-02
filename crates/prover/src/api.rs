use jsonrpsee::{core::RpcResult, proc_macros::rpc};
pub use raiko_lib::input::GuestInput;
use raiko_lib::{
    consts::ChainSpec,
    input::{BlockMetadata, TaikoProverData},
    primitives::mpt::{MptNode, StorageEntry},
};
use reth_primitives::{
    revm_primitives::{Address, Bytes, HashMap},
    Block, Header,
};
use serde::{Deserialize, Serialize};

use crate::guest_input_to_proof_input;

#[rpc(server, client, namespace = "prover")]
pub trait ProverV1Api {
    #[method(name = "genProof")]
    async fn gen_proof(&self, req: ProofRequest) -> RpcResult<ProofResponse>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofRequest {
    pub input: ProofInput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofInput {
    pub l2_block: Block,
    pub parent_header: Header,
    pub chain_spec: ChainSpec,
    pub parent_state_trie: MptNode,
    pub parent_storage: HashMap<Address, StorageEntry>,
    pub contracts: Vec<Bytes>,
    pub ancestor_headers: Vec<Header>,
    pub taiko: ProofTaikoInput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofTaikoInput {
    // Synced L1 header
    pub l1_header: Header,
    // TaikoL1 L2 Block metadata
    pub metadata: BlockMetadata,
    // Taiko prover data
    pub prover_data: TaikoProverData,
}

impl ProofInput {
    pub fn from_guest_input_bytes(data: &[u8]) -> Result<ProofInput, String> {
        let guest_input: GuestInput =
            serde_json::from_slice(data).map_err(|err| err.to_string())?;
        guest_input_to_proof_input(guest_input)
    }
}

pub fn read_guest_input(data: &[u8]) -> Result<GuestInput, serde_json::Error> {
    serde_json::from_slice(data)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofResponse {
    pub version: u64,
    pub data: Bytes,
}

#[cfg(test)]
mod test {
    

    #[test]
    pub fn test_run() {}
}
