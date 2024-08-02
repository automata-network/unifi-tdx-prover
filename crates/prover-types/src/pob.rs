use std::collections::BTreeMap;

use ethers_core::types::Withdrawal;
use serde::{Deserialize, Serialize};

use crate::{Address, HexBytes, H256, U256};

#[derive(Default, Debug, Deserialize, Serialize, Clone)]
pub struct Pob {
    // Blocks within must be continuous and non-repetitive
    pub blocks: Vec<PobBlock>,
    pub data: PobData,
}

#[derive(Default, Debug, Deserialize, Serialize, Clone)]
pub struct PobData {
    pub chain_id: u64,
    // state_root before the first block of the Pob
    pub prev_state_root: H256,
    // block hashes for previous 128 blocks
    pub block_hashes: BTreeMap<u64, H256>,
    // rlp encoded for the mpt nodes has been accessed
    pub mpt_nodes: Vec<HexBytes>,
    // contract codes
    pub codes: Vec<HexBytes>,
}

#[derive(Default, Debug, Deserialize, Serialize, Clone)]
pub struct PobBlock {
    pub hash: Option<H256>,
    pub miner: Option<Address>,
    pub state_root: H256,
    pub number: Option<u64>,
    pub gas_used: U256,
    pub gas_limit: U256,
    pub timestamp: U256,
    pub difficulty: U256,
    pub mix_hash: Option<H256>,
    pub base_fee_per_gas: Option<U256>,
    pub blob_gas_used: Option<U256>,
    pub excess_blob_gas: Option<U256>,

    pub withdrawals: Option<Vec<Withdrawal>>,
    // rlp encoded
    pub transactions: Vec<HexBytes>,
}

