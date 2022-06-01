use ipc_zk::prover::prove;
use ipc_zk::verifier::{verify, TraceProof};
use log::info;
use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::sync::Once;
use types::eth::{BlockResult, BlockResultWrapper, ZkProof};
use zkevm::prover::Prover;
use zkevm::utils::{get_block_result_from_file, load_or_create_params, load_or_create_seed};
use zkevm::verifier::Verifier;

const PROVE_SOCKET: &str = "/tmp/prover.sock";
const VERIFY_SOCKET: &str = "/tmp/verifier.sock";
const PARAMS_NAME: &str = "./tests/test_params";
const SEED_NAME: &str = "./tests/test_seed";
const TRACE_PATH: &str = "./tests/trace.json";
const EVM_PROOF_PATH: &str = "./tests/proofs/evm_proof";
const STATE_PROOF_PATH: &str = "./tests/proofs/state_proof";
static ENV_LOGGER: Once = Once::new();

fn init() {
    ENV_LOGGER.call_once(env_logger::init);
    let _ = load_or_create_params(PARAMS_NAME).expect("failed to load or create params");
    let _ = load_or_create_seed(SEED_NAME).expect(" failed to load or create seed");
}

#[cfg(feature = "prove")]
#[test]
fn test_prove() {
    dotenv::dotenv().ok();
    init();

    let zk_prover = Prover::from_fpath(PARAMS_NAME, SEED_NAME);
    // Start the IPC prover in a separate thread.
    info!("spawning thread to run ipc-prover");
    std::thread::spawn(|| {
        prove(zk_prover, PROVE_SOCKET);
    });

    let block_result = get_block_result_from_file(TRACE_PATH);
    let buf = create_block_trace_bytes(block_result);

    info!("sending block trace");
    let mut stream = UnixStream::connect(Path::new(PROVE_SOCKET)).unwrap();
    stream
        .write_all(&buf)
        .expect("write trace into socket failed");

    info!("reading proving result");
    let proof = read_zk_proof(&mut stream);
    let is_evm_proof = proof.evm_transcript == read_proof_from_file(EVM_PROOF_PATH);
    let is_state_proof = proof.state_transcript == read_proof_from_file(STATE_PROOF_PATH);
    assert!(is_evm_proof && is_state_proof)
}

#[test]
fn test_verify() {
    dotenv::dotenv().ok();
    init();

    let zk_verifier = Verifier::from_fpath(PARAMS_NAME);

    // Start the IPC verifier in a separate thread.
    info!("spawning thread to run ipc-verifier");
    std::thread::spawn(|| {
        verify(zk_verifier, VERIFY_SOCKET);
    });

    let block_result = get_block_result_from_file(TRACE_PATH);

    let evm_proof = if Path::new(EVM_PROOF_PATH).exists() {
        read_proof_from_file(EVM_PROOF_PATH)
    } else {
        let prover = Prover::from_fpath(PARAMS_NAME, SEED_NAME);
        let evm_proof = prover
            .create_evm_proof(&block_result)
            .expect("failed to create evm proof");
        let mut f = File::create(EVM_PROOF_PATH).unwrap();
        f.write_all(evm_proof.as_slice())
            .expect("write evm proof failed");
        evm_proof
    };

    let state_proof = if Path::new(STATE_PROOF_PATH).exists() {
        read_proof_from_file(STATE_PROOF_PATH)
    } else {
        let prover = Prover::from_fpath(PARAMS_NAME, SEED_NAME);
        let state_proof = prover
            .create_state_proof(&block_result)
            .expect("failed to create state proof");
        let mut f = File::create(STATE_PROOF_PATH).unwrap();
        f.write_all(state_proof.as_slice())
            .expect("write state proof failed");
        state_proof
    };

    info!(
        "evm proof length = {}, state proof length = {}",
        evm_proof.len(),
        state_proof.len()
    );

    let buf = create_trace_proof_bytes(evm_proof, state_proof, block_result);

    // Send buf to unix socket, and await a response
    info!("sending proof");
    let mut stream = UnixStream::connect(Path::new(VERIFY_SOCKET)).unwrap();
    stream
        .write_all(&buf)
        .expect("write proof into socket failed");

    info!("reading verifying result");
    let mut response = [0u8; 1];
    stream
        .read_exact(&mut response)
        .expect("read verifying result from socket failed");

    assert_eq!(response[0], 1);
}

fn create_trace_proof_bytes(
    evm_proof: Vec<u8>,
    state_proof: Vec<u8>,
    block_result: BlockResult,
) -> Vec<u8> {
    let mut buf = vec![];
    let trace_proof = TraceProof {
        trace: block_result,
        proof: ZkProof {
            id: 0,
            evm_transcript: evm_proof,
            state_transcript: state_proof,
        },
    };
    let mut bytes = serde_json::to_vec(&trace_proof).unwrap();
    let bytes_len = (bytes.len() as u32).to_be_bytes();
    buf.append(&mut bytes_len.to_vec());
    buf.append(&mut bytes);
    buf
}

fn create_block_trace_bytes(block_result: BlockResult) -> Vec<u8> {
    let mut buf = vec![];
    let block_trace = BlockResultWrapper {
        id: 0,
        block_result,
    };
    let mut bytes = serde_json::to_vec(&block_trace).unwrap();
    let bytes_len = (bytes.len() as u32).to_be_bytes();
    buf.append(&mut bytes_len.to_vec());
    buf.append(&mut bytes);
    buf
}

fn read_zk_proof(socket: &mut UnixStream) -> ZkProof {
    let mut buf = [0u8; 4];
    socket
        .read_exact(&mut buf)
        .expect("read length from socket failed");

    let data_len = u32::from_be_bytes(buf);
    let mut buf = vec![0u8; data_len as usize];
    socket
        .read_exact(&mut buf)
        .expect("read data from socket failed");

    serde_json::from_slice::<ZkProof>(&buf).expect("failed to deserialize zk-proof")
}

fn read_proof_from_file(path: &str) -> Vec<u8> {
    let mut f = File::open(path).expect("Open evm-proof file failed");
    let mut buf = Vec::new();
    f.read_to_end(&mut buf)
        .expect("Read evm proof from file failed");
    buf
}
