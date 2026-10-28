fn main() {
    let src = std::fs::read_to_string(std::env::args().nth(1).unwrap()).unwrap();
    match xidlc::fmt::format_typescript_source(&src) {
        Ok(out) => print!("{out}"),
        Err(e) => {
            eprintln!("ERR: {e}");
            std::process::exit(1);
        }
    }
}
