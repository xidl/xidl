mod cli;
mod driver;
mod error;
mod fmt;
mod generate;
mod jsonrpc;
mod macros;

extern crate self as xidlc;

fn main() {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_writer(std::io::stderr)
        .init();

    if let Err(err) = cli::run() {
        tracing::error!("idlc: {err}");
        std::process::exit(1);
    }
}
