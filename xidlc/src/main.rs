mod cli;
mod driver;
mod error;
mod generate;
mod ipc;
mod jsonrpc;

fn main() {
    if let Err(err) = cli::run() {
        eprintln!("idlc: {err}");
        std::process::exit(1);
    }
}
