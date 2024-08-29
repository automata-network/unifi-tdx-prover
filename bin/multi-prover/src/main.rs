use actix_web::{
    post,
    web::{Data, Json, JsonConfig},
    App, HttpResponse, HttpServer, Responder,
};
use prover::{guest_input_to_proof_input, Keypair, ProofRequest, Prover};
use prover::{GuestInput, ProverV1ApiServer};

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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    let kp = Keypair::new();
    HttpServer::new(move || {
        let prover = Prover::new(kp.clone());

        App::new()
            .app_data(JsonConfig::default().limit(100 << 20))
            .app_data(Data::new(prover))
            .service(gen_proof)
            .service(gen_proof_by_guest_input)
    })
    .bind("127.0.0.1:3000")?
    .run()
    .await
}
