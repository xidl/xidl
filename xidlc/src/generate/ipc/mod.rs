use crate::error::{IdlcError, Result};
use crate::generate::jsonrpc::JsonRpcClient;
use crate::generate::GeneratedFile;
use std::io::BufReader;
use std::net::TcpListener;
use std::process::{Command, Stdio};
use xidl_parser::hir;

pub struct SpawnedJsonRpc {
    child: std::process::Child,
    client: JsonRpcClient<BufReader<std::net::TcpStream>, std::net::TcpStream>,
}

impl SpawnedJsonRpc {
    pub fn generate(
        mut self,
        spec: &hir::Specification,
        input: &str,
    ) -> Result<Vec<GeneratedFile>> {
        let files = self.client.generate(spec, Some(input))?;
        let status = self.child.wait()?;
        if !status.success() {
            return Err(IdlcError::rpc(format!("plugin exited with {status}")));
        }
        Ok(files)
    }
}

pub fn spawn_jsonrpc(lang: &str) -> Result<SpawnedJsonRpc> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?.to_string();
    let exe = format!("xidl-{lang}");
    let child = Command::new(&exe)
        .arg("--jsonrpc-fd")
        .arg(&addr)
        .stderr(Stdio::inherit())
        .spawn()?;

    let (stream, _) = listener.accept()?;
    let writer = stream.try_clone()?;
    let client = JsonRpcClient::new(BufReader::new(stream), writer);
    Ok(SpawnedJsonRpc { child, client })
}
