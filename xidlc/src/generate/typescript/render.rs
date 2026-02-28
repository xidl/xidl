use crate::error::IdlcResult;

pub use crate::generate::typescript::renderer::TypescriptRenderer;

#[derive(Default)]
pub struct TypescriptRenderOutput {
    pub types: String,
    pub zod: String,
    pub client: String,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum TsMode {
    #[default]
    All,
    InterfaceOnly,
    TypesOnly,
}

impl TsMode {
    pub fn allows_interfaces(self) -> bool {
        matches!(self, TsMode::All | TsMode::InterfaceOnly)
    }

    pub fn allows_types(self) -> bool {
        matches!(self, TsMode::All | TsMode::TypesOnly)
    }
}

pub trait TypescriptRender {
    fn render(
        &self,
        file_stem: &str,
        renderer: &TypescriptRenderer,
        mode: TsMode,
    ) -> IdlcResult<TypescriptRenderOutput>;
}
