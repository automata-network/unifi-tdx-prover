use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use raiko_lib::primitives::mpt::{MptNode, StateAccount};
use reth_evm::execute::ProviderError;
use reth_primitives::{
    keccak256,
    revm_primitives::{db::Database, Account, AccountInfo, Bytecode},
    Address, B256, U256,
};

use crate::{BlockDataProvider, ExecutionError};

pub struct MemDB<P: BlockDataProvider> {
    provider: Arc<P>,
    contracts: BTreeMap<B256, Bytecode>,
}

impl<P: BlockDataProvider> MemDB<P> {
    pub fn new(provider: Arc<P>) -> Self {
        let mut db = Self {
            provider,
            contracts: BTreeMap::new(),
        };
        db.init();
        db
    }

    fn init(&mut self) {
        for code in self.provider.contract_codes() {
            let hash = keccak256(code);
            self.contracts.insert(hash, Bytecode::new_raw(code.clone()));
        }
    }

    fn get_acc(&mut self, addr: Address) -> Result<Option<StateAccount>, ProviderError> {
        let Some(acc) = self.provider.get_acc::<StateAccount>(addr)? else {
            return Ok(None);
        };
        Ok(Some(acc))
    }

    pub fn apply_changes(
        &self,
        changes: HashMap<Address, Account>,
    ) -> Result<MptNode, ExecutionError> {
        let mut account_touched = 0;
        let mut storage_touched = 0;

        let mut state_trie = self.provider.state_trie().clone();
        for (address, account) in changes {
            if account.status.is_empty() {
                continue;
            }

            // compute the index of the current account in the state trie
            let state_trie_index = keccak256(address).0;

            if account.is_selfdestructed() {
                state_trie
                    .delete(&state_trie_index)
                    .map_err(ExecutionError::DeleteAccount(&address))?;
                continue;
            }

            account_touched += 1;

            let state_storage = &account.storage;
            let storage_root = {
                // getting a mutable reference is more efficient than calling remove
                // every account must have an entry, even newly created accounts
                let mut storage_trie = self.provider.storage_state_trie(address).clone();

                // for cleared accounts always start from the empty trie
                if account.is_selfdestructed() {
                    storage_trie.clear();
                }

                // apply all new storage entries for the current account (address)
                for (key, value) in state_storage {
                    let storage_trie_index = keccak256(key.to_be_bytes::<32>()).0;
                    if value.present_value().is_zero() {
                        storage_trie
                            .delete(&storage_trie_index)
                            .map_err(ExecutionError::DeleteStorage(key))?;
                    } else {
                        storage_trie
                            .insert_rlp(&storage_trie_index, value.present_value())
                            .map_err(ExecutionError::SetStorage(key))?;
                    }
                }

                storage_touched += 1;

                storage_trie.hash()
            };

            let state_account = StateAccount {
                nonce: account.info.nonce,
                balance: account.info.balance,
                storage_root,
                code_hash: account.info.code_hash,
            };
            state_trie
                .insert_rlp(&state_trie_index, state_account)
                .map_err(ExecutionError::SetAccount(&address))?;
        }

        log::debug!(
            "apply changes: account_touched: {}, storage_touched: {}",
            account_touched,
            storage_touched,
        );

        Ok(state_trie)
    }
}

impl<P: BlockDataProvider> Database for MemDB<P> {
    type Error = ProviderError;
    fn basic(&mut self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        let acc = self.get_acc(address)?;
        let acc = acc.map(|acc| AccountInfo {
            balance: acc.balance,
            nonce: acc.nonce,
            code_hash: acc.code_hash,
            code: None,
        });
        Ok(acc)
    }

    fn block_hash(&mut self, number: U256) -> Result<B256, Self::Error> {
        Ok(self.provider.block_hash(number.to()))
    }

    fn code_by_hash(&mut self, code_hash: B256) -> Result<Bytecode, Self::Error> {
        self.contracts
            .get(&code_hash)
            .cloned()
            .ok_or(ProviderError::StateForHashNotFound(code_hash))
    }

    fn storage(&mut self, address: Address, index: U256) -> Result<U256, Self::Error> {
        let acc = self.get_acc(address)?.expect("should have storage");

        let result = self
            .provider
            .get_slot::<U256>(address, acc.storage_root, index)?
            .unwrap_or_default();
        Ok(result)
    }
}
