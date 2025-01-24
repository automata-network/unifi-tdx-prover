use std::collections::BTreeMap;

use alloy_sol_types::SolValue;
use raiko_lib::{
    input::{
        ontake::{BaseFeeConfig, BlockMetadataV2},
        BlockMetadata, BlockProposedFork, GuestInput,
    },
    primitives::mpt::MptNode,
};
use reth_evm::execute::ProviderError;
use reth_evm_ethereum::taiko::TaikoData;
use reth_primitives::{keccak256, Address, Block, Bytes, Header, B256, U256};
use serde::{Deserialize, Serialize};

use crate::{ProofInput, ProofTaikoInput};
use executor::BlockDataProvider;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Pob {
    pub block: Block,
    pub data: PobData,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PobData {
    pub chain_id: u64,
    // state_root before the first block of the Pob
    pub prev_state_root: B256,
    // block hashes for previous 128 blocks
    pub block_hashes: BTreeMap<u64, B256>,
    // rlp encoded for the mpt nodes has been accessed
    pub mpt_nodes: MptNode,
    // temportary fields
    pub storage_mpt_nodes: BTreeMap<Address, MptNode>,
    // contract codes
    pub codes: Vec<Bytes>,

    // required by TaikoData
    pub l2_contract: Option<Address>,
    pub l1_header: Header,
    pub l2_parent_header: Header,

    // required by proof
    pub graffiti: B256,
    pub l1_contract: Option<Address>,
    pub prover: Address,               // input.taiko.prover_data.prover
    pub block_meta: BlockMetaDataFork, // input.taiko.metadata
    pub base_fee_config: BaseFeeConfig,
}

impl From<ProofInput> for Pob {
    fn from(value: ProofInput) -> Self {
        let mut block_hashes = BTreeMap::new();
        for item in &value.ancestor_headers {
            block_hashes.insert(item.number, item.hash_slow());
        }
        block_hashes.insert(value.parent_header.number, value.parent_header.hash_slow());

        let mut storage_mpt_nodes = BTreeMap::new();
        for (addr, (mpt, _)) in value.parent_storage {
            storage_mpt_nodes.insert(addr, mpt);
        }

        let data = PobData {
            chain_id: value.chain_spec.chain_id,
            prev_state_root: value.parent_header.state_root,
            block_hashes,
            mpt_nodes: value.parent_state_trie,
            storage_mpt_nodes,
            codes: value.contracts,
            l1_header: value.taiko.l1_header,
            l1_contract: value.chain_spec.l1_contract,
            l2_contract: value.chain_spec.l2_contract,
            l2_parent_header: value.parent_header,
            graffiti: value.taiko.prover_data.graffiti,
            prover: value.taiko.prover_data.prover,
            base_fee_config: get_base_fee_config(&value.taiko.metadata),
            block_meta: value.taiko.metadata,
        };
        Self {
            block: value.l2_block,
            data,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BlockMetaDataFork {
    None,
    Hekla(BlockMetadata),
    Ontake(BlockMetadataV2),
}

pub fn guest_input_to_proof_input(input: GuestInput) -> Result<ProofInput, String> {
    Ok(ProofInput {
        l2_block: input.block,
        parent_header: input.parent_header,
        chain_spec: input.chain_spec,
        parent_state_trie: input.parent_state_trie,
        parent_storage: input.parent_storage,
        contracts: input.contracts,
        ancestor_headers: input.ancestor_headers,
        taiko: ProofTaikoInput {
            l1_header: input.taiko.l1_header,
            metadata: select_block_meta(&input.taiko.block_proposed),
            prover_data: input.taiko.prover_data,
        },
    })
}

pub fn guest_input_to_proof_inputs(inputs: Vec<GuestInput>) -> Result<Vec<ProofInput>, String> {
    let mut output = Vec::with_capacity(inputs.len());
    for input in inputs {
        output.push(guest_input_to_proof_input(input)?);
    }
    Ok(output)
}

pub fn select_block_meta(block: &BlockProposedFork) -> BlockMetaDataFork {
    match block {
        BlockProposedFork::Nothing => BlockMetaDataFork::None,
        BlockProposedFork::Hekla(b) => BlockMetaDataFork::Hekla(b.meta.clone()),
        BlockProposedFork::Ontake(b) => BlockMetaDataFork::Ontake(b.meta.clone()),
    }
}

pub fn get_base_fee_config(meta: &BlockMetaDataFork) -> BaseFeeConfig {
    match meta {
        BlockMetaDataFork::Ontake(b) => BaseFeeConfig {
            adjustmentQuotient: b.baseFeeConfig.adjustmentQuotient,
            sharingPctg: b.baseFeeConfig.sharingPctg,
            gasIssuancePerSecond: b.baseFeeConfig.gasIssuancePerSecond,
            minGasExcess: b.baseFeeConfig.minGasExcess,
            maxGasIssuancePerBlock: b.baseFeeConfig.maxGasIssuancePerBlock,
        },
        _ => BaseFeeConfig::default(),
    }
}

pub fn meta_hash(bm: &BlockMetaDataFork) -> B256 {
    match bm {
        BlockMetaDataFork::None => keccak256(vec![]).into(),
        BlockMetaDataFork::Hekla(ref meta) => keccak256(meta.abi_encode()).into(),
        BlockMetaDataFork::Ontake(ref meta) => keccak256(meta.abi_encode()).into(),
    }
}

impl From<GuestInput> for Pob {
    fn from(value: GuestInput) -> Self {
        let mut block_hashes = BTreeMap::new();
        for item in &value.ancestor_headers {
            block_hashes.insert(item.number, item.hash_slow());
        }
        let mut storage_mpt_nodes = BTreeMap::new();
        for (addr, (mpt, _)) in value.parent_storage {
            storage_mpt_nodes.insert(addr, mpt);
        }

        let data = PobData {
            chain_id: value.chain_spec.chain_id,
            prev_state_root: value.parent_header.state_root,
            block_hashes,
            mpt_nodes: value.parent_state_trie,
            storage_mpt_nodes,
            codes: value.contracts,
            l1_header: value.taiko.l1_header,
            l1_contract: value.chain_spec.l1_contract,
            l2_parent_header: value.parent_header,
            l2_contract: value.chain_spec.l2_contract,
            graffiti: value.taiko.prover_data.graffiti,
            block_meta: select_block_meta(&value.taiko.block_proposed),
            prover: value.taiko.prover_data.prover,
            base_fee_config: unsafe {
                std::mem::transmute(value.taiko.block_proposed.base_fee_config())
            },
        };
        Self {
            block: value.block,
            data,
        }
    }
}

impl BlockDataProvider for Pob {
    type ExtData = TaikoData;

    fn ext_data(&self) -> Self::ExtData {
        TaikoData {
            l2_contract: self.data.l2_contract.unwrap_or_default(),
            l1_header: self.data.l1_header.clone(),
            parent_header: self.data.l2_parent_header.clone(),
            base_fee_config: unsafe { std::mem::transmute(self.data.base_fee_config.clone()) },
        }
    }

    fn chain_id(&self) -> u64 {
        self.data.chain_id
    }

    fn block_hash(&self, number: u64) -> B256 {
        self.data
            .block_hashes
            .get(&number)
            .cloned()
            .unwrap_or_default()
    }

    fn block(&self) -> &Block {
        &self.block
    }

    fn contract_codes(&self) -> &[Bytes] {
        &self.data.codes
    }

    fn state_trie(&self) -> &MptNode {
        &self.data.mpt_nodes
    }

    fn storage_state_trie(&self, addr: Address) -> &MptNode {
        self.data
            .storage_mpt_nodes
            .get(&addr)
            .expect("should have the storage trie")
    }

    fn get_acc<T: alloy_rlp::Decodable>(&self, addr: Address) -> Result<Option<T>, ProviderError> {
        let key = keccak256(addr);
        let key = key.as_slice();
        let result = self.data.mpt_nodes.get_rlp::<T>(key).map_err(|err| {
            ProviderError::RPC(format!("get account[{:?}] fail: {}", addr, err.to_string()))
        })?;
        Ok(result)
    }

    fn get_slot<T: alloy_rlp::Decodable>(
        &self,
        key: Address,
        root: B256,
        slot: U256,
    ) -> Result<Option<T>, ProviderError> {
        let Some(storage_trie) = self.data.storage_mpt_nodes.get(&key) else {
            return Ok(None);
        };
        let storage_hash = storage_trie.hash();
        if storage_hash != root {
            return Err(ProviderError::RPC(format!(
                "slot[addr={:?},index={}] storage root mismatch: {:?}, want {:?}",
                key, slot, storage_hash, root,
            )));
        }
        let slot_key = keccak256(slot.to_be_bytes::<32>());
        Ok(storage_trie.get_rlp(slot_key.as_slice()).map_err(|err| {
            ProviderError::RPC(format!(
                "fetch slot[addr={:?},index={}] fail: {}",
                key,
                slot,
                err.to_string()
            ))
        })?)
    }
}
