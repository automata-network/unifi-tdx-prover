use std::{path::PathBuf, time::Duration};

use alloy::{
    primitives::Address,
    sol_types::SolType,
};
use awc::Client;
use base::ProverRegistryStub::{Context, Proof};
use clap::Parser;
use prover::{meta_hash, Pob, ProofRequest, ProofResponse, SignedPoe};
use serde::Deserialize;

#[derive(Parser, Deserialize, Debug)]
pub struct ProofSubmitter {
    #[clap(short, default_value = "")]
    #[serde(skip)]
    pub config: String,

    #[clap(long, env = "PRIVATE_KEY", default_value = "")]
    pub private_key: String,
    #[clap(long, env = "L1_ENDPOINT", default_value = "")]
    pub l1_endpoint: String,
    #[clap(
        long,
        env = "PROVER_REGISTRY",
        default_value = "0x0000000000000000000000000000000000000000"
    )]
    pub prover_registry: Address,

    #[clap(long, default_value = "http://127.0.0.1:20300")]
    pub prover_url: String,

    #[serde(skip)]
    pub input: PathBuf,
}

impl ProofSubmitter {
    pub fn merge(&mut self, rhs: ProofSubmitter) {
        if self.private_key == "" {
            self.private_key = rhs.private_key;
        }
        if self.l1_endpoint == "" {
            self.l1_endpoint = rhs.l1_endpoint;
        }
        if self.prover_registry == Address::default() {
            self.prover_registry = rhs.prover_registry;
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    let mut app = ProofSubmitter::parse();
    if app.config != "" {
        let data = std::fs::read(&app.config).unwrap();
        app.merge(serde_json::from_slice(&data).unwrap());
    }
    let eth = base::Eth::dial(&app.l1_endpoint, Some(&app.private_key)).unwrap();

    let registry = base::ProverRegistry::new(eth, app.prover_registry, Some(Duration::from_secs(60)));

    let client = Client::default();
    let mut input: ProofRequest =
        serde_json::from_slice(&std::fs::read(&app.input).unwrap()).unwrap();
    input.input.chain_spec.l1_contract = Some(app.prover_registry);

    let contract_chain_id = registry.chain_id().await.unwrap();
    if contract_chain_id != input.input.chain_spec.chain_id {
        panic!("chain id mismatch, contract:{}, local:{}", contract_chain_id, input.input.chain_spec.chain_id);
    }

    let pob: Pob = input.input.clone().into();

    let url = format!("{}/v1/gen_proof", app.prover_url);

    let mut response = client
        .post(url)
        .timeout(Duration::from_secs(60))
        .send_json(&input)
        .await
        .unwrap();
    let body = response.body().await.unwrap();
    if !response.status().is_success() {
        panic!("{:?}", body);
    }

    let response: ProofResponse = serde_json::from_slice(&body).unwrap();

    let poe = SignedPoe::abi_decode(&response.data, true).unwrap();

    let ctx = Context {
        prover: input.input.taiko.prover_data.prover,
        metaHash: meta_hash(&input.input.taiko.metadata),
        ..Default::default()
    };



    let contract_signed_msg = registry
        .get_poe_hash(
            unsafe { std::mem::transmute_copy(&poe.poe) },
            ctx.metaHash,
            poe.new_instance,
            ctx.prover,
        )
        .await
        .unwrap();

    assert_eq!(
        contract_signed_msg,
        poe.poe.signed_msg(&pob, poe.new_instance),
        "signed msg mismatch with contract side"
    );

    // let addr = registry
    //     .recover_old_instance(ctx.clone(), unsafe { std::mem::transmute_copy(&poe) })
    //     .await
    //     .unwrap();
    // println!("addr: {:?}", addr);
    // return Ok(());

    let result = registry
        .verify_proofs(vec![Proof {
            ctx,
            poe: unsafe { std::mem::transmute(poe) },
        }])
        .await;
    println!("{:?}", result);

    Ok(())
}
