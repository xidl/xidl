use clap::Parser;
use xidlc::cli::Cli;
use xidlc::diagnostic::IdlMietteHighlighter;
use xidlc::error::IdlcError;

#[tokio::main]
async fn main() {
    miette::set_hook(Box::new(|_| {
        Box::new(
            miette::MietteHandlerOpts::new()
                .with_syntax_highlighting(IdlMietteHighlighter)
                .build(),
        )
    }))
    .expect("failed to install miette hook");

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    if let Err(err) = Cli::parse().run().await {
        if let IdlcError::Diagnostic(report) = err {
            let report: miette::Report = report.into();
            eprintln!("{:?}", report)
        } else {
            tracing::error!("idlc: {err}");
        }
        std::process::exit(1);
    }
}
