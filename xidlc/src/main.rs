use clap::Parser;
use xidlc::cli::Cli;
use xidlc::error::IdlcError;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
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
