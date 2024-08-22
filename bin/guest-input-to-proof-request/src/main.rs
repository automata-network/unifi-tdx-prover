use std::path::PathBuf;

use prover::ProofRequest;

fn main() {
    let args = std::env::args().collect::<Vec<_>>();

    let guest_input_path = PathBuf::new().join(&args[1]);
    let file_name = guest_input_path.file_name().unwrap().to_str().unwrap();
    let guest_input_prefix = "guest-input-";
    if !file_name.starts_with(guest_input_prefix) {
        panic!("file_name should starts with {:?}", guest_input_prefix);
    }
    let file_name = file_name.trim_start_matches(guest_input_prefix);
    println!("{:?}", guest_input_path.file_name());
    let data = std::fs::read(&guest_input_path).unwrap();
    let guest_input = prover::read_guest_input(&data).unwrap();

    let proof_input = prover::guest_input_to_proof_input(guest_input).unwrap();
    let proof_request = serde_json::to_vec_pretty(&ProofRequest { input: proof_input }).unwrap();

    let dest = guest_input_path
        .parent()
        .unwrap()
        .join(format!("proof-request-{}", file_name));

    std::fs::write(dest, proof_request).unwrap()
}
