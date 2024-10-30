use std::{collections::BTreeMap, time::Duration};

use alloy::primitives::{Bytes, U64};
use base::Base64Bytes;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

pub struct AgentService {
    client: awc::Client,
}

impl AgentService {
    pub fn new() -> Self {
        Self {
            client: awc::Client::default(),
        }
    }

    async fn http_get<T: DeserializeOwned>(
        &self,
        path: &str,
        report_data: &Bytes,
    ) -> Result<T, String> {
        let url = format!(
            "http://127.0.0.1:8000/{}/{}",
            path,
            alloy::hex::encode(report_data)
        );
        let mut response = self
            .client
            .get(&url)
            .timeout(Duration::from_secs(60))
            .send()
            .await
            .map_err(|err| format!("request {:?} fail: {:?}", url, err))?;
        let body = response.body().await.unwrap();
        Ok(serde_json::from_slice(&body).map_err(|err| format!("der fail: {:?}", err))?)
    }

    pub async fn tdx_report_with_tpm(
        &self,
        report_data: &Bytes,
    ) -> Result<AgentServiceResponse, String> {
        self.http_get("tdx-report-with-tpm-extension", report_data)
            .await
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TdxContents {
    pub attestation_report: Base64Bytes,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TpmContents {
    pub quote: Base64Bytes,
    pub raw_sig: Base64Bytes,
    pub pcrs: Option<Pcrs>,
    // Certificate for the Attestation Key. X509 DER Bytes.
    pub ak_cert: Option<Base64Bytes>,
    // Certificate for the Endorsement Key. X509 DER Bytes.
    pub ek_cert: Option<Base64Bytes>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Pcrs {
    pub pcrs: BTreeMap<U64, Base64Bytes>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentServiceResponse {
    // SEV-SNP attestation report
    pub tdx: TdxContents,
    // TPM quote of PCR 10
    pub tpm: Option<TpmContents>,
    // IMA measurement log from `/sys/kernel/security/ima/ascii_runtime_measurements`
    pub ima_measurement: Option<Base64Bytes>,
    // Nonce
    pub nonce: Option<Base64Bytes>,
}

alloy::sol! {
    #[derive(Debug, Default)]

    struct ExtTpmInfo {
        bytes32 pcr10;
        bytes quote;
        bytes signature;
        bytes akDer;
    }}

#[cfg(test)]
mod test {
    use super::AgentServiceResponse;

    #[test]
    fn test_parse_local_agent_response() {
        let data = std::fs::read("../../testdata/local-agent-with-tpm.json").unwrap();
        let data: AgentServiceResponse = serde_json::from_slice(&data).unwrap();
    }
}
