use alloy::{
    primitives::{Address, U256},
    rpc::types::TransactionReceipt,
    sol_types::SolEvent,
};
use ProverRegistryStub::{Proof, ProverRegistryStubErrors};

use crate::{Eth, EthError};

#[derive(Clone)]
pub struct ProverRegistry {
    eth: Eth,
    contract: Address,
}

pub use ProverRegistryStub::{registerCall as RegisterCall, ReportData};

crate::stack_error! {
    name: RegistryError,
    stack_name: RegistryErrorStack,
    error: {
        Revert(ProverRegistryStubErrors, EthError),
        Eth(EthError),
        MissingInstanceIdOnRegister,
    },
    wrap: {
    },
    stack: {}
}

impl From<EthError> for RegistryError {
    fn from(value: EthError) -> Self {
        match value.revert_data::<ProverRegistryStubErrors>() {
            Ok((err, value)) => Self::Revert(err, value),
            Err(err) => Self::Eth(err),
        }
    }
}

impl ProverRegistry {
    pub fn new(eth: Eth, contract: Address) -> Self {
        Self { eth, contract }
    }

    pub async fn chain_id(&self) -> Result<u64, RegistryError> {
        let call = ProverRegistryStub::uniFiChainIdCall {};
        Ok(self.eth.call(self.contract, &call).await?._0)
    }

    pub async fn attest_validity_seconds(&self) -> Result<u64, RegistryError> {
        let call = ProverRegistryStub::attestValiditySecondsCall {};
        Ok(self.eth.call(self.contract, &call).await?._0.to())
    }

    pub fn address(&self) -> Address {
        self.contract
    }

    fn get_event<T: SolEvent + Clone>(receipt: &TransactionReceipt) -> Option<T> {
        for log in receipt.inner.logs() {
            if let Ok(event) = log.log_decode::<T>() {
                return Some(event.data().clone());
            }
        }
        return None;
    }

    pub async fn register<T>(&self, report: T) -> Result<Registration, RegistryError>
    where
        T: Into<RegisterCall>,
    {
        use ProverRegistryStub::*;

        let call = report.into();

        let tx = self.eth.transact(self.contract, &call).await?;
        log::info!("[register] waiting receipt for: {:?}", tx.tx_hash());
        let receipt = tx.get_receipt().await.map_err(EthError::from)?;

        let instance_add = Self::get_event::<InstanceAdded>(&receipt)
            .ok_or(RegistryError::MissingInstanceIdOnRegister)?;

        Ok(Registration {
            address: instance_add.instance,
            instance_id: instance_add.id,
            valid_until: instance_add.validUntil.to(),
        })
    }

    pub async fn verify_proofs(&self, proofs: Vec<Proof>) -> Result<(), RegistryError> {
        use ProverRegistryStub::*;

        let call = verifyProofsCall { _proofs: proofs };
        let tx = self.eth.transact(self.contract, &call).await?;
        log::info!("[verify_proofs] waiting receipt for: {:?}", tx.tx_hash());
        let receipt = tx.get_receipt().await.map_err(EthError::from)?;
        log::info!("receipt: {:?}", receipt);
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct Registration {
    pub address: Address,
    pub instance_id: U256,
    pub valid_until: u64,
}

alloy::sol! {
    #[derive(Debug, Default)]
    ProverRegistryStub,
    r#"[{"type":"function","name":"acceptOwnership","inputs":[],"outputs":[],"stateMutability":"nonpayable"},{"type":"function","name":"addressManager","inputs":[],"outputs":[{"name":"","type":"address","internalType":"address"}],"stateMutability":"view"},{"type":"function","name":"attestValiditySeconds","inputs":[],"outputs":[{"name":"","type":"uint256","internalType":"uint256"}],"stateMutability":"view"},{"type":"function","name":"attestedProvers","inputs":[{"name":"proverInstanceID","type":"uint256","internalType":"uint256"}],"outputs":[{"name":"addr","type":"address","internalType":"address"},{"name":"validUntil","type":"uint256","internalType":"uint256"},{"name":"teeType","type":"uint256","internalType":"uint256"}],"stateMutability":"view"},{"type":"function","name":"attestedReports","inputs":[{"name":"reportHash","type":"bytes32","internalType":"bytes32"}],"outputs":[{"name":"used","type":"bool","internalType":"bool"}],"stateMutability":"view"},{"type":"function","name":"checkProver","inputs":[{"name":"_instanceID","type":"uint256","internalType":"uint256"},{"name":"_proverAddr","type":"address","internalType":"address"}],"outputs":[{"name":"","type":"tuple","internalType":"struct IProverRegistry.ProverInstance","components":[{"name":"addr","type":"address","internalType":"address"},{"name":"validUntil","type":"uint256","internalType":"uint256"},{"name":"teeType","type":"uint256","internalType":"uint256"}]}],"stateMutability":"view"},{"type":"function","name":"impl","inputs":[],"outputs":[{"name":"","type":"address","internalType":"address"}],"stateMutability":"view"},{"type":"function","name":"inNonReentrant","inputs":[],"outputs":[{"name":"","type":"bool","internalType":"bool"}],"stateMutability":"view"},{"type":"function","name":"init","inputs":[{"name":"_owner","type":"address","internalType":"address"},{"name":"_rollupAddressManager","type":"address","internalType":"address"},{"name":"_verifierAddr","type":"address","internalType":"address"},{"name":"_attestValiditySeconds","type":"uint256","internalType":"uint256"},{"name":"_maxBlockNumberDiff","type":"uint256","internalType":"uint256"}],"outputs":[],"stateMutability":"nonpayable"},{"type":"function","name":"lastUnpausedAt","inputs":[],"outputs":[{"name":"","type":"uint64","internalType":"uint64"}],"stateMutability":"view"},{"type":"function","name":"maxBlockNumberDiff","inputs":[],"outputs":[{"name":"","type":"uint256","internalType":"uint256"}],"stateMutability":"view"},{"type":"function","name":"nextInstanceId","inputs":[],"outputs":[{"name":"","type":"uint256","internalType":"uint256"}],"stateMutability":"view"},{"type":"function","name":"owner","inputs":[],"outputs":[{"name":"","type":"address","internalType":"address"}],"stateMutability":"view"},{"type":"function","name":"pause","inputs":[],"outputs":[],"stateMutability":"nonpayable"},{"type":"function","name":"paused","inputs":[],"outputs":[{"name":"","type":"bool","internalType":"bool"}],"stateMutability":"view"},{"type":"function","name":"pendingOwner","inputs":[],"outputs":[{"name":"","type":"address","internalType":"address"}],"stateMutability":"view"},{"type":"function","name":"proxiableUUID","inputs":[],"outputs":[{"name":"","type":"bytes32","internalType":"bytes32"}],"stateMutability":"view"},{"type":"function","name":"register","inputs":[{"name":"_report","type":"bytes","internalType":"bytes"},{"name":"_data","type":"tuple","internalType":"struct IProverRegistry.ReportData","components":[{"name":"addr","type":"address","internalType":"address"},{"name":"teeType","type":"uint256","internalType":"uint256"},{"name":"referenceBlockNumber","type":"uint256","internalType":"uint256"},{"name":"referenceBlockHash","type":"bytes32","internalType":"bytes32"},{"name":"binHash","type":"bytes32","internalType":"bytes32"},{"name":"ext","type":"bytes","internalType":"bytes"}]}],"outputs":[],"stateMutability":"nonpayable"},{"type":"function","name":"reinitialize","inputs":[{"name":"i","type":"uint8","internalType":"uint8"},{"name":"_verifierAddr","type":"address","internalType":"address"},{"name":"_attestValiditySeconds","type":"uint256","internalType":"uint256"},{"name":"_maxBlockNumberDiff","type":"uint256","internalType":"uint256"}],"outputs":[],"stateMutability":"nonpayable"},{"type":"function","name":"renounceOwnership","inputs":[],"outputs":[],"stateMutability":"nonpayable"},{"type":"function","name":"resolve","inputs":[{"name":"_chainId","type":"uint64","internalType":"uint64"},{"name":"_name","type":"bytes32","internalType":"bytes32"},{"name":"_allowZeroAddress","type":"bool","internalType":"bool"}],"outputs":[{"name":"","type":"address","internalType":"address"}],"stateMutability":"view"},{"type":"function","name":"resolve","inputs":[{"name":"_name","type":"bytes32","internalType":"bytes32"},{"name":"_allowZeroAddress","type":"bool","internalType":"bool"}],"outputs":[{"name":"","type":"address","internalType":"address"}],"stateMutability":"view"},{"type":"function","name":"transferOwnership","inputs":[{"name":"newOwner","type":"address","internalType":"address"}],"outputs":[],"stateMutability":"nonpayable"},{"type":"function","name":"uniFiChainId","inputs":[],"outputs":[{"name":"","type":"uint64","internalType":"uint64"}],"stateMutability":"view"},{"type":"function","name":"unpause","inputs":[],"outputs":[],"stateMutability":"nonpayable"},{"type":"function","name":"upgradeTo","inputs":[{"name":"newImplementation","type":"address","internalType":"address"}],"outputs":[],"stateMutability":"nonpayable"},{"type":"function","name":"upgradeToAndCall","inputs":[{"name":"newImplementation","type":"address","internalType":"address"},{"name":"data","type":"bytes","internalType":"bytes"}],"outputs":[],"stateMutability":"payable"},{"type":"function","name":"verifier","inputs":[],"outputs":[{"name":"","type":"address","internalType":"contract IAttestationVerifier"}],"stateMutability":"view"},{"type":"function","name":"verifyBatchProof","inputs":[{"name":"_ctxs","type":"tuple[]","internalType":"struct IVerifier.ContextV2[]","components":[{"name":"metaHash","type":"bytes32","internalType":"bytes32"},{"name":"blobHash","type":"bytes32","internalType":"bytes32"},{"name":"prover","type":"address","internalType":"address"},{"name":"blockId","type":"uint64","internalType":"uint64"},{"name":"isContesting","type":"bool","internalType":"bool"},{"name":"blobUsed","type":"bool","internalType":"bool"},{"name":"msgSender","type":"address","internalType":"address"},{"name":"tran","type":"tuple","internalType":"struct TaikoData.Transition","components":[{"name":"parentHash","type":"bytes32","internalType":"bytes32"},{"name":"blockHash","type":"bytes32","internalType":"bytes32"},{"name":"stateRoot","type":"bytes32","internalType":"bytes32"},{"name":"graffiti","type":"bytes32","internalType":"bytes32"}]}]},{"name":"_proof","type":"tuple","internalType":"struct TaikoData.TierProof","components":[{"name":"tier","type":"uint16","internalType":"uint16"},{"name":"data","type":"bytes","internalType":"bytes"}]}],"outputs":[],"stateMutability":"pure"},{"type":"function","name":"verifyProof","inputs":[{"name":"_ctx","type":"tuple","internalType":"struct IVerifier.Context","components":[{"name":"metaHash","type":"bytes32","internalType":"bytes32"},{"name":"blobHash","type":"bytes32","internalType":"bytes32"},{"name":"prover","type":"address","internalType":"address"},{"name":"blockId","type":"uint64","internalType":"uint64"},{"name":"isContesting","type":"bool","internalType":"bool"},{"name":"blobUsed","type":"bool","internalType":"bool"},{"name":"msgSender","type":"address","internalType":"address"}]},{"name":"_tran","type":"tuple","internalType":"struct TaikoData.Transition","components":[{"name":"parentHash","type":"bytes32","internalType":"bytes32"},{"name":"blockHash","type":"bytes32","internalType":"bytes32"},{"name":"stateRoot","type":"bytes32","internalType":"bytes32"},{"name":"graffiti","type":"bytes32","internalType":"bytes32"}]},{"name":"_proof","type":"tuple","internalType":"struct TaikoData.TierProof","components":[{"name":"tier","type":"uint16","internalType":"uint16"},{"name":"data","type":"bytes","internalType":"bytes"}]}],"outputs":[],"stateMutability":"nonpayable"},{"type":"function","name":"verifyProofs","inputs":[{"name":"_proofs","type":"tuple[]","internalType":"struct IProverRegistry.Proof[]","components":[{"name":"poe","type":"tuple","internalType":"struct IProverRegistry.SignedPoe","components":[{"name":"transition","type":"tuple","internalType":"struct TaikoData.Transition","components":[{"name":"parentHash","type":"bytes32","internalType":"bytes32"},{"name":"blockHash","type":"bytes32","internalType":"bytes32"},{"name":"stateRoot","type":"bytes32","internalType":"bytes32"},{"name":"graffiti","type":"bytes32","internalType":"bytes32"}]},{"name":"id","type":"uint256","internalType":"uint256"},{"name":"newInstance","type":"address","internalType":"address"},{"name":"signature","type":"bytes","internalType":"bytes"},{"name":"teeType","type":"uint256","internalType":"uint256"}]},{"name":"ctx","type":"tuple","internalType":"struct IVerifier.Context","components":[{"name":"metaHash","type":"bytes32","internalType":"bytes32"},{"name":"blobHash","type":"bytes32","internalType":"bytes32"},{"name":"prover","type":"address","internalType":"address"},{"name":"blockId","type":"uint64","internalType":"uint64"},{"name":"isContesting","type":"bool","internalType":"bool"},{"name":"blobUsed","type":"bool","internalType":"bool"},{"name":"msgSender","type":"address","internalType":"address"}]}]}],"outputs":[],"stateMutability":"nonpayable"},{"type":"event","name":"AdminChanged","inputs":[{"name":"previousAdmin","type":"address","indexed":false,"internalType":"address"},{"name":"newAdmin","type":"address","indexed":false,"internalType":"address"}],"anonymous":false},{"type":"event","name":"BeaconUpgraded","inputs":[{"name":"beacon","type":"address","indexed":true,"internalType":"address"}],"anonymous":false},{"type":"event","name":"Initialized","inputs":[{"name":"version","type":"uint8","indexed":false,"internalType":"uint8"}],"anonymous":false},{"type":"event","name":"InstanceAdded","inputs":[{"name":"id","type":"uint256","indexed":true,"internalType":"uint256"},{"name":"instance","type":"address","indexed":true,"internalType":"address"},{"name":"replaced","type":"address","indexed":false,"internalType":"address"},{"name":"validUntil","type":"uint256","indexed":false,"internalType":"uint256"}],"anonymous":false},{"type":"event","name":"OwnershipTransferStarted","inputs":[{"name":"previousOwner","type":"address","indexed":true,"internalType":"address"},{"name":"newOwner","type":"address","indexed":true,"internalType":"address"}],"anonymous":false},{"type":"event","name":"OwnershipTransferred","inputs":[{"name":"previousOwner","type":"address","indexed":true,"internalType":"address"},{"name":"newOwner","type":"address","indexed":true,"internalType":"address"}],"anonymous":false},{"type":"event","name":"Paused","inputs":[{"name":"account","type":"address","indexed":false,"internalType":"address"}],"anonymous":false},{"type":"event","name":"Unpaused","inputs":[{"name":"account","type":"address","indexed":false,"internalType":"address"}],"anonymous":false},{"type":"event","name":"Upgraded","inputs":[{"name":"implementation","type":"address","indexed":true,"internalType":"address"}],"anonymous":false},{"type":"event","name":"VerifyProof","inputs":[{"name":"proofs","type":"uint256","indexed":false,"internalType":"uint256"}],"anonymous":false},{"type":"error","name":"BLOCK_NUMBER_MISMATCH","inputs":[]},{"type":"error","name":"BLOCK_NUMBER_OUT_OF_DATE","inputs":[]},{"type":"error","name":"FUNC_NOT_IMPLEMENTED","inputs":[]},{"type":"error","name":"INVALID_BLOCK_NUMBER","inputs":[]},{"type":"error","name":"INVALID_PAUSE_STATUS","inputs":[]},{"type":"error","name":"INVALID_PRC10","inputs":[{"name":"pcr10","type":"bytes32","internalType":"bytes32"}]},{"type":"error","name":"INVALID_PROVER_INSTANCE","inputs":[]},{"type":"error","name":"INVALID_REPORT","inputs":[]},{"type":"error","name":"INVALID_REPORT_DATA","inputs":[]},{"type":"error","name":"PROVER_ADDR_MISMATCH","inputs":[{"name":"","type":"address","internalType":"address"},{"name":"","type":"address","internalType":"address"}]},{"type":"error","name":"PROVER_INVALID_ADDR","inputs":[{"name":"","type":"address","internalType":"address"}]},{"type":"error","name":"PROVER_INVALID_INSTANCE_ID","inputs":[{"name":"","type":"uint256","internalType":"uint256"}]},{"type":"error","name":"PROVER_INVALID_PROOF","inputs":[]},{"type":"error","name":"PROVER_OUT_OF_DATE","inputs":[{"name":"","type":"uint256","internalType":"uint256"}]},{"type":"error","name":"PROVER_TYPE_MISMATCH","inputs":[]},{"type":"error","name":"REENTRANT_CALL","inputs":[]},{"type":"error","name":"REPORT_DATA_MISMATCH","inputs":[{"name":"want","type":"bytes32","internalType":"bytes32"},{"name":"got","type":"bytes32","internalType":"bytes32"}]},{"type":"error","name":"REPORT_USED","inputs":[]},{"type":"error","name":"RESOLVER_DENIED","inputs":[]},{"type":"error","name":"RESOLVER_INVALID_MANAGER","inputs":[]},{"type":"error","name":"RESOLVER_UNEXPECTED_CHAINID","inputs":[]},{"type":"error","name":"RESOLVER_ZERO_ADDR","inputs":[{"name":"chainId","type":"uint64","internalType":"uint64"},{"name":"name","type":"bytes32","internalType":"bytes32"}]},{"type":"error","name":"ZERO_ADDRESS","inputs":[]},{"type":"error","name":"ZERO_VALUE","inputs":[]}]"#,
}
