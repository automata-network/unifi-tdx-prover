# unifi-tee-multi-prover


## Getting Started

### start server:
```
$ cargo run --release --bin multi-prover
```

### test with rakio guest input
```
$ curl localhost:3000/debug/gen_proof_by_guest_input -H 'Content-Type: application/json' -d @testdata/guest-input-ethereum-20335518.json
```

### convert guest input to proof request
```
$ cargo run -bin guest-input-to-proof-request testdata/guest-input-ethereum-20335518.json
```

### test with proof request
```
$ curl localhost:3000/v1/gen_proof -H 'Content-Type: application/json' -d @testdata/proof-request-ethereum-20335518.json
```