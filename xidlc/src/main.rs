use clap::Parser;
use xidlc::cli::Cli;
use xidlc::diagnostic::TreeSitterMietteHighlighter;
use xidlc::error::{DiagnosticListError, IdlcError};

#[tokio::main]
async fn main() {
    miette::set_hook(Box::new(|_| {
        Box::new(
            miette::MietteHandlerOpts::new()
                .with_syntax_highlighting(TreeSitterMietteHighlighter::new_idl())
                .build(),
        )
    }))
    .expect("failed to install miette hook");

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    if let Err(err) = Cli::parse().run().await {
        match err {
            IdlcError::Diagnostics(DiagnosticListError { diagnostics }) => {
                for (index, diagnostic) in diagnostics.into_iter().enumerate() {
                    if index > 0 {
                        eprintln!();
                    }
                    let report: miette::Report = diagnostic.into();
                    eprintln!("{:?}", report);
                }
            }
            other => {
                tracing::error!("idlc: {other}");
            }
        }
        std::process::exit(1);
    }
}
