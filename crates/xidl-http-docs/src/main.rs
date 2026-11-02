use xidl_http_docs::DocumentationPlugin;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let plugin = DocumentationPlugin::from_environment()?;
    xidlc::jsonrpc::serve_stdio(plugin)?;
    Ok(())
}
