use crate::error::IdlcResult;
use crate::generate::rust_jsonrpc::definition::render_module_body;
use crate::generate::rust_jsonrpc::{JsonRpcRender, JsonRpcRenderOutput, JsonRpcRenderer};
use xidl_parser::jsonrpc_hir::JsonRpcHirDocument;

impl JsonRpcRender for JsonRpcHirDocument {
    fn render(&self, renderer: &JsonRpcRenderer) -> IdlcResult<JsonRpcRenderOutput> {
        let source = render_module_body(&self.interfaces, renderer)?;
        Ok(JsonRpcRenderOutput { source })
    }
}
