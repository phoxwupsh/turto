use clap::Parser;
use turto::{cli::Cli, log::setup_log};

fn main() {
    let _guard = match setup_log() {
        Ok(guard) => guard,
        Err(err) => {
            eprintln!("failed to setup log: {:#}", err);
            return;
        }
    };
    let cli = Cli::parse();
    cli.run();
}
