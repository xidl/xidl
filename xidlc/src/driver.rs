use crate::cli::CliArgs;
use crate::error::{IdlcError, Result};
use crate::generate::GeneratedFile;
use crate::ipc;
use crate::jsonrpc::JsonRpcClient;
use std::io::BufReader;
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::fs;
use std::path::Path;

pub fn run(args: CliArgs) -> Result<()> {
    fs::create_dir_all(&args.out_dir)?;

    for input in args.inputs {
        let source = fs::read_to_string(&input)?;
        let typed = xidl_parser::parser::parser_text(&source)?;
        let hir = xidl_parser::hir::Specification::from(typed);
        let files = generate_for_lang(&args.lang, &hir, &input)?;
        write_files(&args.out_dir, files)?;
    }

    Ok(())
}

fn generate_for_lang(
    lang: &str,
    hir: &xidl_parser::hir::Specification,
    input: &Path,
) -> Result<Vec<GeneratedFile>> {
    let input_str = input.to_string_lossy();
    if lang == "c" {
        generate_with_c_server(hir, &input_str)
    } else {
        ipc::spawn_jsonrpc(lang)?.generate(hir, &input_str)
    }
}

fn generate_with_c_server(
    hir: &xidl_parser::hir::Specification,
    input: &str,
) -> Result<Vec<GeneratedFile>> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?;
    let server = thread::spawn(move || {
        let (stream, _) = listener.accept()?;
        let reader = BufReader::new(stream.try_clone()?);
        let writer = stream;
        crate::generate::c::serve_jsonrpc(reader, writer)
    });

    let client_stream = TcpStream::connect(addr)?;
    let reader = BufReader::new(client_stream.try_clone()?);
    let writer = client_stream;
    let mut client = JsonRpcClient::new(reader, writer);
    let files = client.generate(hir, Some(input))?;

    let server_result = server
        .join()
        .map_err(|_| IdlcError::rpc("c server thread panicked"))?;
    server_result?;
    Ok(files)
}

fn write_files(out_dir: &Path, files: Vec<GeneratedFile>) -> Result<()> {
    for file in files {
        let path = out_dir.join(file.filename);
        fs::write(path, file.filecontent)?;
    }
    Ok(())
}
