use std::{path::PathBuf, process::Command};

use alloy::{
    primitives::{keccak256, Bytes, B256, U256, U64},
    sol_types::SolValue,
};
use async_trait::async_trait;
use base::ReportData;

use crate::{AgentService, ExtTpmInfo, ReportBuilder};

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
    async fn generate_quote(&self, rp: ReportData) -> Result<(Bytes, Bytes), String> {
        let report_data: Bytes = keccak256(&rp.abi_encode()).to_vec().into();

        let response = self.las.tdx_report_with_tpm(&report_data).await?;
        let tpm = response
            .tpm
            .ok_or_else(|| format!("required tpm from agent service response"))?;
        let pcrs = tpm
            .pcrs
            .ok_or_else(|| format!("required pcrs from agent service response"))?;
        let pcr10 = pcrs
            .pcrs
            .get(&U64::from_limbs([10]))
            .ok_or_else(|| format!("required pcr10 from agent service response"))?;
        if pcr10.len() != 32 {
            return Err(format!("pcr10.len() != 32: {:?}", pcr10));
        }
        let ak_der = match tpm.ak_cert {
            Some(n) => n.0,
            None => Bytes::new(),
        };

        let ext = ExtTpmInfo {
            pcr10: B256::from_slice(&pcr10),
            akDer: ak_der,
            quote: tpm.quote.0,
            signature: tpm.raw_sig.0,
        };

        Ok((
            response.tdx.attestation_report.0,
            ext.abi_encode().into(),
        ))
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
    async fn generate_quote(&self, rp: ReportData) -> Result<(Bytes, Bytes), String> {
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

        Ok((output, Bytes::new()))
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
