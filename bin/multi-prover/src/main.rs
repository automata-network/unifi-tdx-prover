use std::{
    path::PathBuf,
    time::{Duration, Instant, SystemTime},
};

use actix_web::{
    post,
    rt::{spawn, time::sleep},
    web::{Data, Json, JsonConfig},
    App, HttpResponse, HttpServer, Responder,
};
use alloy::primitives::{Address, U256};
use base::{Eth, Keypair, ProverRegistry};
use clap::Parser;
use prover::{guest_input_to_proof_input, guest_input_to_proof_inputs, MultiProofRequest, ProofRequest, Prover, RpcMultiProofRequest};
use prover::{GuestInput, ProverV1ApiServer};
use raiko_core::interfaces::ProofRequest as RpcProofRequest;
use serde::Deserialize;
use tee::{AttestationReport, ReportBuilder};

#[post("/debug/gen_proof_by_guest_input")]
async fn gen_proof_by_guest_input(prover: Data<Prover>, req: Json<GuestInput>) -> impl Responder {
    let req = ProofRequest {
        input: guest_input_to_proof_input(req.0).unwrap(),
    };
    match prover.gen_proof(req).await {
        Ok(n) => HttpResponse::Ok().json(n),
        Err(err) => HttpResponse::BadRequest().json(err),
    }
}

#[post("/v1/gen_proof")]
async fn gen_proof(prover: Data<Prover>, req: Json<ProofRequest>) -> impl Responder {
    match prover.gen_proof(req.0).await {
        Ok(n) => HttpResponse::Ok().json(n),
        Err(err) => HttpResponse::BadRequest().json(err),
    }
}

#[post("/v1/get_proof")]
async fn get_proof(prover: Data<Prover>, req: Json<RpcProofRequest>) -> impl Responder {
    let req_data = serde_json::to_string(&req.0);
    let block_number = req.block_number;
    log::info!("req: {:?}", req_data);

    let start = Instant::now();

    let guest_input = prover.get_proof(req.0).await;

    let gen_proof_instant = Instant::now();

    let proof_request = ProofRequest {
        input: guest_input_to_proof_input(guest_input).unwrap(),
    };

    let result = match prover.prove(proof_request) {
        Ok(n) => HttpResponse::Ok().json(n),
        Err(err) => {
            log::error!("err: {:?}", err);
            HttpResponse::BadRequest().json(err)
        }
    };

    log::info!("gen proof time: {:?}, proving time: {:?}, total: {:?}", gen_proof_instant-start, gen_proof_instant.elapsed(), start.elapsed());
    result
}

#[post("/v1/get_proofs")]
async fn get_proofs(prover: Data<Prover>, req: Json<RpcMultiProofRequest>) -> impl Responder {
    let start = Instant::now();

    let guest_inputs = match prover.get_proofs(req.0).await {
        Ok(n) => n,
        Err(err) => return HttpResponse::BadRequest().json(err),
    };

    let gen_proof_instant = Instant::now();

    let proof_request = MultiProofRequest {
        input: guest_input_to_proof_inputs(guest_inputs).unwrap(),
    };

    let result = match prover.prove_multi(proof_request).await {
        Ok(n) => HttpResponse::Ok().json(n),
        Err(err) => {
            log::error!("err: {:?}", err);
            HttpResponse::BadRequest().json(err)
        }
    };

    log::info!("gen proof time: {:?}, proving time: {:?}, total: {:?}", gen_proof_instant-start, gen_proof_instant.elapsed(), start.elapsed());
    result
}

#[derive(Debug, Parser, Deserialize)]
pub struct MultiProver {
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
    #[clap(long, env = "LISTEN", default_value = "127.0.0.1:20300")]
    pub listen: String,
    #[clap(long, default_value = "1800")]
    pub attestation_pre_expire_secs: u64,
    #[clap(long, default_value = "8")]
    pub worker_num: usize,
}

impl MultiProver {
    pub fn merge(&mut self, rhs: MultiProver) {
        if self.private_key == "" {
            self.private_key = rhs.private_key;
        }
        if self.l1_endpoint == "" {
            self.l1_endpoint = rhs.l1_endpoint;
        }
        if self.prover_registry == Address::default() {
            self.prover_registry = rhs.prover_registry;
        }
        if self.listen == "127.0.0.1:20300" && rhs.listen != "" {
            self.listen = rhs.listen
        }
        if self.attestation_pre_expire_secs == 1800 && rhs.attestation_pre_expire_secs > 0 {
            self.attestation_pre_expire_secs = rhs.attestation_pre_expire_secs
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    let mut mp = MultiProver::parse();
    if mp.config != "" {
        let data = std::fs::read(&mp.config).unwrap();
        mp.merge(serde_json::from_slice(&data).unwrap());
    }

    let kp = Keypair::new();

    #[cfg(feature = "tdx")]
    let quote_builder = tee::TdxQuoteLocalAgentBuilder::new();
    #[cfg(not(feature = "tdx"))]
    let quote_builder = tee::MockBuilder::new();

    let tee_type = quote_builder.tee_type();
    if false {
        let prover = Prover::new(kp.clone(), mp.prover_registry, tee_type, mp.worker_num);
        kp.rotate().commit(U256::from_limbs_slice(&[1]));
        let data = std::fs::read(
            PathBuf::new()
                .join("testdata")
                .join("proof-request-unifi-testnet-48.json"),
            // .join("proof-request-taiko-a7-848185.json"),
        )
        .unwrap();
        let data = serde_json::from_slice(&data).unwrap();
        let data = prover.gen_proof(data).await.unwrap();
        dbg!(data);

        return Ok(());
    }

    let client = base::Eth::dial(&mp.l1_endpoint, Some(&mp.private_key)).unwrap();
    let register_timeout = Some(Duration::from_secs(120));
    let registry = ProverRegistry::new(client.clone(), mp.prover_registry, register_timeout);

    let _attestation_loop_handle = spawn(attestation_loop(
        quote_builder,
        client,
        kp.clone(),
        registry,
        mp.attestation_pre_expire_secs,
    ));

    HttpServer::new(move || {
        let prover = Prover::new(kp.clone(), mp.prover_registry, tee_type, mp.worker_num);

        App::new()
            .app_data(JsonConfig::default().limit(100 << 20))
            .app_data(Data::new(prover))
            .service(gen_proof)
            .service(gen_proof_by_guest_input)
            .service(get_proof)
    })
    .bind(mp.listen)?
    .run()
    .await
}

async fn attestation_loop<B: ReportBuilder>(
    quote_builder: B,
    client: Eth,
    kp: Keypair,
    registry: ProverRegistry,
    attestation_pre_expire_secs: u64,
) {
    let err_retry = Duration::from_secs(5);
    loop {
        let new_key = kp.rotate();
        let report = match AttestationReport::build(&quote_builder, &client, &new_key).await {
            Ok(report) => report,
            Err(err) => {
                log::error!(
                    "generate attestation report fail: {:?}, retry in {:?}",
                    err,
                    err_retry
                );
                sleep(err_retry).await;
                continue;
            }
        };

        let registration = match registry.register(report).await {
            Ok(n) => n,
            Err(err) => {
                log::error!(
                    "register on ProverRegistry[{:?}] fail: {:?}",
                    registry.address(),
                    err,
                );
                sleep(err_retry).await;
                continue;
            }
        };
        if registration.address != new_key.address() {
            panic!(
                "register on ProverRegistry[{:?}] fail: address mismatch, want: {:?}, got: {:?}",
                registry.address(),
                new_key.address(),
                registration.address,
            );
        }
        new_key.commit(registration.instance_id);
        let next_attestation_time = registration.valid_until - attestation_pre_expire_secs;

        log::info!(
            "registration successfully: {:?}, next attestation: {:?}",
            registration,
            next_attestation_time
        );

        sleep_until(next_attestation_time).await
    }
}

async fn sleep_until(ts: u64) {
    let epoch = (SystemTime::now().duration_since(SystemTime::UNIX_EPOCH))
        .unwrap()
        .as_secs();
    sleep(Duration::from_secs(ts - epoch)).await
}
