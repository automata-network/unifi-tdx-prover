use raiko_lib::primitives::mpt::{self, Error as MptError};
use reth_evm::execute::{BlockExecutionError, BlockValidationError};
use reth_primitives::{Address, B256, U256};

base::stack_error! {
    name: DataProviderError,
    stack_name: DataProviderErrorStack,
    error: {
        UnsupportChainId(u64, Vec<u64>),
    },
    wrap: {
        MptError(MptError),
    },
    stack: {
    }
}

pub type DataProviderResult<T> = Result<T, DataProviderError>;

base::stack_error! {
    name: ExecutionError,
    stack_name: ExecutionErrorStack,
    error: {
        NotAllTransactionExecuted { remote: Vec<usize>, local: Vec<usize> },
        StateRootMismatch{ remote: B256, local: B256 },
    },
    wrap: {
        DataProvider(DataProviderError),
        BlockValidation(BlockValidationError),
        BlockExecution(BlockExecutionError),
        Mpt(mpt::Error),
    },
    stack: {
        DeleteAccount(addr: Address),
        DeleteStorage(key: U256),
        SetStorage(key: U256),
        SetAccount(addr: Address),
        ApplyChanges(),
        ExecuteBlock(),
    }
}

pub type ExecutionResult<T> = Result<T, ExecutionError>;
