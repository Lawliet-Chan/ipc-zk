use log::{debug, info};
use std::fs;
use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use types::eth::{BlockResultWrapper, ZkProof};
use zkevm::prover::Prover;

/// Runs the IPC prover.
pub fn run(params_file: &str, seed_file: &str, socket_file: &str) {
    info!("starting preliminary setup for proving");

    let zk_prover = Prover::from_fpath(params_file, seed_file);

    let socket = Path::new(socket_file);

    // Delete old socket if present
    if socket.exists() {
        debug!("old socket file removed");
        fs::remove_file(&socket).expect("should be able to clear out old socket file");
    }

    info!("starting server");

    // Start a server on the unix socket
    let listener = UnixListener::bind(&socket).expect("should be able to bind to unix socket");

    let (mut socket, addr) = listener
        .accept()
        .expect("should be able to accept a connection");

    info!("server started on {:?}", addr);

    loop {
        let block_trace = read_block_trace(&mut socket);
        let block_result = &block_trace.block_result;

        let evm_proof = zk_prover
            .create_evm_proof(block_result)
            .expect("evm prove failed");

        let state_proof = zk_prover
            .create_state_proof(block_result)
            .expect("state prove failed");

        let zk_proof = ZkProof {
            id: block_trace.id,
            evm_transcript: evm_proof,
            state_transcript: state_proof,
        };

        let data = create_zk_proof_bytes(&zk_proof);
        socket
            .write_all(data.as_slice())
            .expect("should be able to write to the sequencer");
    }
}

fn read_block_trace(socket: &mut UnixStream) -> BlockResultWrapper {
    let mut buf = [0u8; 4];
    socket
        .read_exact(&mut buf)
        .expect("read length from socket failed");

    let data_len = u32::from_be_bytes(buf);
    let mut buf = vec![0u8; data_len as usize];
    socket
        .read_exact(&mut buf)
        .expect("read data from socket failed");

    serde_json::from_slice::<BlockResultWrapper>(&buf).expect("failed to deserialize trace-proof")
}

fn create_zk_proof_bytes(zk_proof: &ZkProof) -> Vec<u8> {
    let mut buf = vec![];
    let mut bytes = serde_json::to_vec(zk_proof).expect("encode zk-proof failed");
    let mut data_len = u32::to_be_bytes(bytes.len() as u32).to_vec();
    buf.append(&mut data_len);
    buf.append(&mut bytes);
    buf
}
