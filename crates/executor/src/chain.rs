use raiko_lib::builder::UNIFI_TESTNET;
use reth_chainspec::{
    ChainSpec, ChainSpecBuilder, HOLESKY, MAINNET, TAIKO_A7, TAIKO_DEV, TAIKO_MAINNET,
};
use std::{collections::BTreeMap, sync::Arc};

lazy_static::lazy_static! {
    pub static ref CHAIN_LIST: BTreeMap<u64, Arc<ChainSpec>> = chain_map(available_chain_list());
}

fn available_chain_list() -> Vec<Arc<ChainSpec>> {
    vec![
        Arc::new(
            ChainSpecBuilder::default()
                .chain(MAINNET.chain)
                .genesis(MAINNET.genesis.clone())
                .cancun_activated()
                .build(),
        ),
        HOLESKY.clone(),
        TAIKO_A7.clone(),
        TAIKO_DEV.clone(),
        TAIKO_MAINNET.clone(),
        UNIFI_TESTNET.clone(),
    ]
}

fn chain_map(list: Vec<Arc<ChainSpec>>) -> BTreeMap<u64, Arc<ChainSpec>> {
    let mut chain_list = BTreeMap::new();
    for chain in list {
        chain_list.insert(chain.chain().id(), chain.clone());
    }
    chain_list
}
