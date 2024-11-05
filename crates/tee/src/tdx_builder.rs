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
    async fn generate_ext(&self) -> Result<Bytes, String> {
        pub const TPM_ALG_SHA1: u16 = 0x0004;
        pub const TPM_ALG_SHA256: u16 = 0x000b;
        let report_data = B256::default().0.to_vec().into();

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
        let pcr10 = match pcrs.hash {
            TPM_ALG_SHA1 => {
                if pcr10.len() != 20 {
                    return Err(format!("pcr10.len() != 20: {:?}", pcr10));
                }
                let mut val = [0_u8; 32];
                val[12..].copy_from_slice(pcr10);
                B256::from_slice(&val)
            }
            TPM_ALG_SHA256 => {
                if pcr10.len() != 32 {
                    return Err(format!("pcr10.len() != 32: {:?}", pcr10));
                }
                B256::from_slice(&pcr10)
            }
            hash => return Err(format!("unknown pcr hash: {}", hash)),
        };
        let ak_der = match tpm.ak_cert {
            Some(n) => n.0,
            None => Bytes::new(),
        };

        let ext = ExtTpmInfo {
            pcr10,
            akDer: ak_der,
            quote: tpm.quote.0,
            signature: tpm.raw_sig.0,
        };
        println!("{:?}", ext);
        Ok(ext.abi_encode().into())
    }

    async fn generate_quote(&self, rp: ReportData) -> Result<Bytes, String> {
        let report_data: Bytes = keccak256(&rp.abi_encode()).to_vec().into();
        let response = self.las.tdx_report_with_tpm(&report_data).await?;
        // TODO: check whether tpm env changed
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
    async fn generate_ext(&self) -> Result<Bytes, String> {
        Ok(Bytes::new())
    }

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
        U256::from_limbs_slice(&[201])
    }
}

impl TdxQuoteBuilder {
    pub fn new(bin: PathBuf) -> Self {
        Self { bin }
    }
}
