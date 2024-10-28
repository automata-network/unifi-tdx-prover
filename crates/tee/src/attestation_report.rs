use alloy::primitives::{keccak256, Address, Bytes, B256, U256};

use async_trait::async_trait;
use base::{Eth, Keypair, RegisterCall, ReportData};

#[derive(Clone, Debug)]
pub struct AttestationReport {
    pub report: Bytes,
    pub address: Address,
    pub reference_block_hash: B256,
    pub reference_block_number: U256,
    pub bin_hash: B256,
    pub tee_type: U256,
}

#[async_trait(?Send)]
pub trait ReportBuilder {
    async fn generate_quote(&self, rp: ReportData) -> Result<Bytes, String>;
    fn tee_type(&self) -> U256;
}

impl AttestationReport {
    pub async fn build<B>(builder: &B, eth: &Eth, sk: &Keypair) -> Result<Self, String>
    where
        B: ReportBuilder,
    {
        let (number, hash) = eth
            .select_reference_block()
            .await
            .map_err(|err| format!("{:?}", err))?;

        let vars = std::env::args().collect::<Vec<String>>();
        let bin_data = std::fs::read(&vars[0]).unwrap();

        let mut report = Self {
            address: sk.address(),
            report: Bytes::new(),
            reference_block_hash: hash,
            reference_block_number: number,
            tee_type: builder.tee_type(),
            bin_hash: keccak256(&bin_data),
        };

        let call: RegisterCall = report.clone().into();
        report.report = builder.generate_quote(call._data).await?;

        Ok(report)
    }
}

impl From<AttestationReport> for RegisterCall {
    fn from(value: AttestationReport) -> Self {
        RegisterCall {
            _report: value.report,
            _data: ReportData {
                addr: value.address,
                teeType: value.tee_type,
                referenceBlockHash: value.reference_block_hash,
                referenceBlockNumber: value.reference_block_number,
                binHash: value.bin_hash,
                ext: Bytes::new(),
            },
        }
    }
}
