use std::{path::PathBuf, process::Command};

use alloy::{
    primitives::{keccak256, Bytes, U256},
    sol_types::SolValue,
};
use base::ReportData;

use crate::ReportBuilder;

#[derive(Debug, Clone)]
pub struct TdxQuoteBuilder {
    bin: PathBuf,
}

impl ReportBuilder for TdxQuoteBuilder {
    fn generate_quote(&self, rp: ReportData) -> Bytes {

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
        let output = cmd.output().unwrap();

        let start_off = match output.stdout.iter().position(|n| *n == '\n' as u8) {
            Some(idx) => idx + 1,
            None => 0,
        };
        let output: Bytes = output.stdout[start_off..start_off + 4936].to_vec().into();

        output
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
