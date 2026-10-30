mod definition;
mod integer;
mod names;
mod packages;
mod render;
mod scope;
mod spec;

use crate::error::IdlcResult;
use crate::generate::utils::go_package_name;
use crate::jsonrpc::{Artifact, ArtifactFile};
pub use render::GoRenderer;
use serde::Serialize;
use std::path::Path;
use xidl_parser::hir;
use xidl_parser::hir::ParserProperties;

#[cfg(feature = "gen-go-rest")]
pub(crate) use definition::go_field_name;
pub(crate) use names::GoNames;
pub(crate) use packages::{GoImport, GoPackages};
pub(crate) use scope::GoScope;

pub fn generate_with_properties(
    spec: &hir::Specification,
    input_path: &Path,
    properties: &ParserProperties,
) -> IdlcResult<Vec<Artifact>> {
    let allow_versioned_modules = properties
        .get("allow_versioned_modules")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    if !allow_versioned_modules {
        crate::generate::versioning::reject_versioned_modules("go", properties)?;
    }
    let stem = input_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("output");
    // A unit keeps the directory its IDL path names, so a package generated
    // from `pkg/a.idl` is `pkg/a.go` with package name `pkg`.
    let unit_path = properties
        .get(crate::units::UNIT_PATH_PROPERTY)
        .and_then(serde_json::Value::as_str);
    let package = properties
        .get("package")
        .and_then(|v| v.as_str())
        .map(go_package_name)
        .unwrap_or_else(|| match unit_path {
            Some(path) => go_package_name(&package_name_of(path, stem)),
            None => go_package_name(stem),
        });
    let filename = match unit_path {
        Some(path) => format!("{}.go", path.trim_end_matches(".idl")),
        None => format!("{package}.go"),
    };
    let source = spec::render_spec(spec, &package, properties)?;
    Ok(vec![Artifact::new_file(ArtifactFile {
        path: filename,
        content: source,
    })])
}

/// Package name a unit path implies: its directory, or its file stem.
fn package_name_of(path: &str, stem: &str) -> String {
    match path.rsplit_once('/') {
        Some((dir, _)) => dir
            .rsplit('/')
            .next()
            .filter(|name| !name.is_empty())
            .unwrap_or(stem)
            .to_string(),
        None => stem.to_string(),
    }
}

pub(crate) struct GoCodegen;

impl crate::jsonrpc::Codegen for GoCodegen {
    fn get_engine_version(&self) -> Result<String, crate::jsonrpc::RpcError> {
        Ok("*".to_string())
    }

    fn get_properties(&self) -> Result<ParserProperties, crate::jsonrpc::RpcError> {
        Ok(crate::macros::hashmap! {
            "enable_interfaces" => true,
            "enable_render_header" => true,
            "enable_metadata" => true
        })
    }

    fn generate(
        &self,
        input_hir: crate::jsonrpc::CodegenInput,
        input: String,
        props: ::xidl_parser::hir::ParserProperties,
    ) -> Result<Vec<Artifact>, crate::jsonrpc::RpcError> {
        let hir = input_hir.into_rpc_hir();
        generate_with_properties(&hir, Path::new(&input), &props)
            .map_err(crate::jsonrpc::RpcError::from)
    }
}

#[derive(Default)]
pub(crate) struct GoRenderState {
    pub(crate) uses_context: bool,
}

pub(crate) struct GoRenderContext {
    pub(crate) package_name: String,
    pub(crate) properties: ParserProperties,
    pub(crate) state: GoRenderState,
    pub(crate) packages: GoPackages,
    names: GoNames,
    renderer: GoRenderer,
    pub(crate) body: String,
}

impl GoRenderContext {
    pub(crate) fn new(
        package_name: String,
        properties: ParserProperties,
        spec: &hir::Specification,
    ) -> IdlcResult<Self> {
        let packages = GoPackages::of(&properties);
        let renderer = GoRenderer::new(&properties)?;
        Ok(Self {
            package_name,
            properties,
            state: GoRenderState::default(),
            packages,
            names: GoNames::collect(spec),
            renderer,
            body: String::new(),
        })
    }

    /// Resolves types and values from one IDL declaration scope.
    pub(crate) fn scope<'a>(&'a self, path: &'a [String]) -> GoScope<'a> {
        GoScope::new(&self.names, &self.packages, path)
    }

    pub(crate) fn push_template<S: Serialize>(&mut self, name: &str, ctx: &S) -> IdlcResult<()> {
        let rendered = self.render_template(name, ctx)?;
        self.body.push_str(&rendered);
        Ok(())
    }

    pub(crate) fn render_template<S: Serialize>(&self, name: &str, ctx: &S) -> IdlcResult<String> {
        self.renderer.render_template(name, ctx)
    }

    pub(crate) fn finish(self, blocks: Vec<String>) -> IdlcResult<String> {
        let body = blocks.concat();
        let output = GoRenderOutput {
            package_name: self.package_name,
            uses_context: self.state.uses_context,
            imports: self.packages.imports(),
            body,
        };
        self.renderer.render_spec(&output)
    }
}

#[derive(Serialize)]
pub struct GoRenderOutput {
    package_name: String,
    uses_context: bool,
    imports: Vec<GoImport>,
    body: String,
}
pub(crate) enum ParamDirection {
    In,
    Out,
    InOut,
}
