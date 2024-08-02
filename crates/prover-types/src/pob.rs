use std::{collections::BTreeMap, convert::Infallible};

use ethers_core::types::{Block, PreStateMode, Transaction, Withdrawal};
use revm::{
    primitives::{keccak256, AccountInfo, Bytecode},
    Database, DatabaseRef,
};
use serde::{Deserialize, Serialize};

use crate::{Address, HexBytes, H256, U256};

#[derive(Serialize, Deserialize, Debug)]
pub struct Pob {
    pub blocks: Vec<PobBlock>,
    pub data: PobData,
}

#[derive(Default, Debug, Deserialize, Serialize, Clone)]
pub struct PobData {
    pub chain_id: u64,
    // state_root before the first block of this Pob
    pub prev_state_root: H256,
    pub block_hashes: BTreeMap<u64, H256>,
    pub mpt_nodes: Vec<HexBytes>,
    pub codes: Vec<HexBytes>,
}

#[derive(Default, Debug, Deserialize, Serialize, Clone)]
pub struct PobBlock {
    pub hash: Option<H256>,
    pub miner: Option<Address>,
    pub state_root: H256,
    pub number: Option<U64>,
    pub gas_used: U256,
    pub gas_limit: U256,
    pub timestamp: U256,
    pub difficulty: U256,
    pub mix_hash: Option<H256>,
    pub base_fee_per_gas: Option<U256>,
    pub blob_gas_used: Option<U256>,
    pub excess_blob_gas: Option<U256>,
    // rlp encoded
    pub transactions: Vec<HexBytes>,
    pub withdrawals: Option<Vec<Withdrawal>>,
}

