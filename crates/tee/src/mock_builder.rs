use alloy::primitives::{Bytes, U256};
use base::ReportData;
use rand::RngCore;

use crate::ReportBuilder;

pub struct MockBuilder {
    ty: U256,
}

impl MockBuilder {
    pub fn new() -> Self {
        Self {
            ty: U256::from(201),
        }
    }
}

impl ReportBuilder for MockBuilder {
    fn generate_quote(&self, _: ReportData) -> Bytes {
        let mut report = [0_u8; 1024];
        rand::thread_rng().fill_bytes(&mut report);
        report.into()
    }

    fn tee_type(&self) -> U256 {
        self.ty
    }
}
