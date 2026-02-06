use clap::Parser;
use xidlc::cli::Cli;
use xidlc::error::IdlcError;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    if let Err(err) = Cli::parse().run().await {
        if let IdlcError::Diagnostic(report) = err {
            eprintln!("{report:?}");
        } else {
            tracing::error!("idlc: {err}");
        }
        std::process::exit(1);
    }
}
