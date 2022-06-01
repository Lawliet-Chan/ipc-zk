use log::{debug, info};
use serde_derive::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use types::eth::{BlockResult, ZkProof};
use zkevm::verifier::Verifier;

/// Runs the IPC verifier.
pub fn run(params_file: &str, socket_file: &str) {
    info!("starting preliminary setup for verifying");

    let zk_verifier = Verifier::from_fpath(params_file);

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
        let trace_proof = read_trace_proof(&mut socket);
        info!("verifying evm proof");
        let evm_verified =
            zk_verifier.verify_evm_proof(trace_proof.proof.evm_transcript, &trace_proof.trace);

        info!("verifying state proof");
        let state_verified =
            zk_verifier.verify_state_proof(trace_proof.proof.state_transcript, &trace_proof.trace);

        info!("writing response {}", evm_verified && state_verified);
        socket
            .write_all(&[(evm_verified && state_verified) as u8])
            .expect("should be able to write to the sequencer");
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TraceProof {
    pub trace: BlockResult,
    pub proof: ZkProof,
}

fn read_trace_proof(socket: &mut UnixStream) -> TraceProof {
    let mut buf = [0u8; 4];
    socket
        .read_exact(&mut buf)
        .expect("read length from socket failed");

    let data_len = u32::from_be_bytes(buf);
    let mut buf = vec![0u8; data_len as usize];
    socket
        .read_exact(&mut buf)
        .expect("read data from socket failed");

    serde_json::from_slice::<TraceProof>(&buf).expect("failed to deserialize trace-proof")
}
