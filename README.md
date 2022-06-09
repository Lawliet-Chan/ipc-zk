# IPC-ZK
This repo contains `ipc-prover` and  `ipc-verifier`.  
`ipc-prover` accpets the block-traces from `Roller` through unix socket, then generates the zk-proof and send back.  
`ipc-verifier` accepts the zk-proof from `Scroll` through unix socket and verify it, then send back the verification result.   



