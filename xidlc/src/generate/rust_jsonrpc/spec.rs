use crate::error::IdlcResult;
use crate::generate::rust_jsonrpc::definition::render_module_body;
use crate::generate::rust_jsonrpc::{JsonRpcRender, JsonRpcRenderOutput, JsonRpcRenderer};
use itertools::Itertools;
use xidl_parser::hir;

impl JsonRpcRender for hir::Specification {
    fn render(&self, renderer: &JsonRpcRenderer) -> IdlcResult<JsonRpcRenderOutput> {
        let defs = self.0.iter().collect_vec();
        let body = render_module_body(&defs, renderer)?;
        Ok(JsonRpcRenderOutput { source: vec![body] })
    }
}
