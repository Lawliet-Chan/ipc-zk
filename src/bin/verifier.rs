use ipc_zk::verifier::run;
use structopt::StructOpt;

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
    run(&args.params_file, &args.socket_file);
}
