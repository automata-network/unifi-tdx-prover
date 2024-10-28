use std::{path::PathBuf, process::Command};

use alloy::{
    primitives::{keccak256, Bytes, U256},
    sol_types::SolValue,
};
use async_trait::async_trait;
use base::ReportData;

use crate::{AgentService, ReportBuilder};

pub struct TdxQuoteLocalAgentBuilder {
    las: AgentService,
}

impl TdxQuoteLocalAgentBuilder {
    pub fn new() -> Self {
        Self {
            las: AgentService::new(),
        }
    }
}

#[async_trait(?Send)]
impl ReportBuilder for TdxQuoteLocalAgentBuilder {
    async fn generate_quote(&self, rp: ReportData) -> Result<Bytes, String> {
        let report_data: Bytes = keccak256(&rp.abi_encode()).to_vec().into();
        
        let response = self.las.tdx_report_with_tpm(&report_data).await?;

        Ok(response.tdx.attestation_report.0)
    }

    fn tee_type(&self) -> U256 {
        U256::from_limbs_slice(&[1])
    }
}

#[derive(Debug, Clone)]
pub struct TdxQuoteBuilder {
    bin: PathBuf,
}

#[async_trait(?Send)]
impl ReportBuilder for TdxQuoteBuilder {
    async fn generate_quote(&self, rp: ReportData) -> Result<Bytes, String> {
        let mut report_data = [0_u8; 64];
        report_data[32..].copy_from_slice(&keccak256(&rp.abi_encode()).0);

        let mut cmd = Command::new(&self.bin);
        cmd.args([
            "-in",
            &alloy::hex::encode(report_data),
            "-inform",
            "hex",
            "-outform",
            "bin",
        ]);
        let output = cmd.output().map_err(|err| format!("{:?}", err))?;

        let start_off = match output.stdout.iter().position(|n| *n == '\n' as u8) {
            Some(idx) => idx + 1,
            None => 0,
        };
        let output: Bytes = output.stdout[start_off..start_off + 4936].to_vec().into();

        Ok(output)
    }

    fn tee_type(&self) -> U256 {
        U256::from_limbs_slice(&[1])
    }
}

impl TdxQuoteBuilder {
    pub fn new(bin: PathBuf) -> Self {
        Self { bin }
    }
}
