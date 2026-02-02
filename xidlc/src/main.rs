mod cli;
mod driver;
mod error;
mod fmt;
mod generate;
mod highlight;
mod jsonrpc;
mod macros;

extern crate self as xidlc;

use clap::Parser;

fn main() {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_writer(std::io::stderr)
        .init();

    if let Err(err) = cli::Cli::parse().run() {
        if let crate::error::IdlcError::Diagnostic(report) = err {
            eprintln!("{report:?}");
        } else {
            tracing::error!("idlc: {err}");
        }
        std::process::exit(1);
    }
}
