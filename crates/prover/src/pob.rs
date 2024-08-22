use std::collections::BTreeMap;

use raiko_lib::{
    consts::VerifierType, input::GuestInput, primitives::mpt::MptNode,
    protocol_instance::ProtocolInstance,
};
use reth_evm::execute::ProviderError;
use reth_evm_ethereum::taiko::TaikoData;
use reth_primitives::{keccak256, Address, Block, Bytes, Header, B256, U256};
use serde::{Deserialize, Serialize};

use crate::ProofInput;
use executor::BlockDataProvider;

#[derive(Default, Debug, Deserialize, Serialize, Clone)]
pub struct Pob {
    pub block: Block,
    pub data: PobData,
}

#[derive(Default, Debug, Deserialize, Serialize, Clone)]
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
    pub l1_header: Option<Header>,
    pub l2_parent_header: Header,
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
            l1_header: value.l1_header,
            l2_contract: value.chain_spec.l2_contract,
            l2_parent_header: value.parent_header,
        };
        Self {
            block: value.l2_block,
            data,
        }
    }
}

pub fn guest_input_to_proof_input(input: GuestInput) -> Result<ProofInput, String> {
    let pi = ProtocolInstance::new(&input, &input.block.header, VerifierType::SGX)
        .map_err(|err| err.to_string())?;
    Ok(ProofInput {
        meta: pi.block_metadata,
        l2_block: input.block,
        parent_header: input.parent_header,
        chain_spec: input.chain_spec,
        parent_state_trie: input.parent_state_trie,
        parent_storage: input.parent_storage,
        contracts: input.contracts,
        ancestor_headers: input.ancestor_headers,
        l1_header: Some(input.taiko.l1_header),
    })
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
            l1_header: Some(value.taiko.l1_header),
            l2_parent_header: value.parent_header,
            l2_contract: value.chain_spec.l2_contract,
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
            l1_header: self.data.l1_header.clone().unwrap_or_default(),
            parent_header: self.data.l2_parent_header.clone(),
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
