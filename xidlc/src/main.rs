mod cli;
mod driver;
mod error;
mod generate;
mod jsonrpc;

extern crate self as xidlc;

fn main() {
    if let Err(err) = cli::run() {
        eprintln!("idlc: {err}");
        std::process::exit(1);
    }
}
