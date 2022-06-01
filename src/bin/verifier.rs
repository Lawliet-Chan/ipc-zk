use ipc_zk::verifier::verify;
use structopt::StructOpt;
use zkevm::verifier::Verifier;

#[derive(Clone, Debug, StructOpt)]
struct Args {
    /// Location of the setup parameters file.
    #[structopt(short, long, default_value = "params")]
    params_file: String,
    /// Location of the Unix socket file.
    #[structopt(short, long, default_value = "/tmp/verifier.sock")]
    socket_file: String,
}

fn main() {
    // Init logging
    dotenv::dotenv().ok();
    env_logger::init();

    let args = Args::from_args();
    let zk_verifier = Verifier::from_fpath(&args.params_file);
    verify(zk_verifier, &args.socket_file);
}
