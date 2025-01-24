use alloy::primitives::B256;

#[derive(Debug)]
pub enum HashAlgo {
    SHA256,
}

#[derive(Debug)]
pub struct IMALog {
    pub log_hash: B256,
    pub data_hash: B256,
    pub data_hash_algo: HashAlgo,
    pub name: String,
}