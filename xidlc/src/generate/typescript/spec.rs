use crate::error::IdlcResult;
use crate::generate::typescript::definition::render_typescript;
use crate::generate::typescript::{
    TsMode, TypescriptRender, TypescriptRenderOutput, TypescriptRenderer,
};
use xidl_parser::hir;

impl TypescriptRender for hir::Specification {
    fn render(
        &self,
        file_stem: &str,
        renderer: &TypescriptRenderer,
        mode: TsMode,
    ) -> IdlcResult<TypescriptRenderOutput> {
        render_typescript(self, file_stem, renderer, mode)
    }
}
