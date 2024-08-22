use std::{collections::HashMap, sync::Arc};

use raiko_lib::primitives::mpt::MptNode;
use reth_chainspec::ChainSpec;
use reth_evm::{
    execute::{BlockValidationError, Executor, ProviderError},
    BundleState,
};
use reth_evm_ethereum::{execute::EthExecutorProvider, taiko::TaikoData};
use reth_primitives::{
    revm_primitives::{Account, AccountStatus},
    Address, Block, BlockWithSenders, Bytes, B256, U256,
};

use crate::{
    DataProviderError, DataProviderResult, ExecutionError, ExecutionResult, MemDB, CHAIN_LIST,
};

pub trait BlockDataProvider {
    type ExtData;

    fn chain_id(&self) -> u64;
    fn block(&self) -> &Block;
    fn contract_codes(&self) -> &[Bytes];
    fn block_hash(&self, number: u64) -> B256;
    fn state_trie(&self) -> &MptNode;
    fn storage_state_trie(&self, addr: Address) -> &MptNode;

    fn get_acc<T: alloy_rlp::Decodable>(&self, key: Address) -> Result<Option<T>, ProviderError>;
    fn get_slot<T: alloy_rlp::Decodable>(
        &self,
        key: Address,
        root: B256,
        slot: U256,
    ) -> Result<Option<T>, ProviderError>;

    fn ext_data(&self) -> Self::ExtData;

    fn get_chain_spec(&self) -> DataProviderResult<Arc<ChainSpec>> {
        let chain_id = self.chain_id();
        if let Some(spec) = CHAIN_LIST.get(&chain_id) {
            Ok(spec.clone())
        } else {
            Err(DataProviderError::UnsupportChainId(
                chain_id,
                CHAIN_LIST.keys().cloned().collect(),
            ))
        }
    }
}

pub struct BlockExecutor<P: BlockDataProvider> {
    provider: Arc<P>,
}

impl<P> BlockExecutor<P>
where
    P: BlockDataProvider<ExtData = TaikoData>,
{
    pub fn new(provider: Arc<P>) -> Self {
        Self { provider }
    }

    fn collect_changes(&self, state: BundleState) -> HashMap<Address, Account> {
        state
            .state
            .into_iter()
            .map(|(address, bundle_account)| {
                let mut account = Account {
                    info: bundle_account.info.unwrap_or_default(),
                    storage: bundle_account.storage,
                    status: AccountStatus::default(),
                };
                account.mark_touch();
                if bundle_account.status.was_destroyed() {
                    account.mark_selfdestruct();
                }
                if bundle_account.original_info.is_none() {
                    account.mark_created();
                }
                (address, account)
            })
            .collect()
    }

    pub fn execute(&self) -> ExecutionResult<BlockWithSenders> {
        let chain_spec = self.provider.get_chain_spec()?;
        let db = MemDB::new(self.provider.clone());
        let executor = EthExecutorProvider::ethereum(chain_spec)
            .eth_executor(db)
            .taiko_data(self.provider.ext_data())
            .optimistic(false);

        let block = self.provider.block().clone();

        let block = block
            .with_recovered_senders()
            .ok_or(BlockValidationError::SenderRecoveryError)?;

        let input = (&block, U256::ZERO).into();
        let result = executor
            .execute(input)
            .map_err(ExecutionError::ExecuteBlock())?;
        let changes = self.collect_changes(result.state);

        // make sure all txs are executed
        let tx_idx = (0..block.body.len()).collect::<Vec<_>>();
        if tx_idx != result.valid_transaction_indices {
            return Err(ExecutionError::NotAllTransactionExecuted {
                remote: tx_idx,
                local: result.valid_transaction_indices,
            });
        }

        let new_state_trie = result
            .db
            .database
            .apply_changes(changes)
            .map_err(ExecutionError::ApplyChanges())?;

        if block.header.state_root != new_state_trie.hash() {
            return Err(ExecutionError::StateRootMismatch {
                remote: block.header.state_root,
                local: new_state_trie.hash(),
            });
        }

        Ok(block)
    }
}
