use alloy::primitives::{Bytes, U256};
use async_trait::async_trait;
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

#[async_trait(?Send)]
impl ReportBuilder for MockBuilder {
    async fn generate_ext(&self) -> Result<Bytes, String> {
        Ok(Bytes::new())
    }

    async fn generate_quote(&self, _: ReportData) -> Result<Bytes, String> {
        let mut report = [0_u8; 1024];
        rand::thread_rng().fill_bytes(&mut report);
        Ok(report.into())
    }

    fn tee_type(&self) -> U256 {
        self.ty
    }
}
