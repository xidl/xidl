use super::generate_session::CodegenSession;
use super::session_pool::SessionPool;
use super::{ArgsGenerate, Driver, Generator};
use crate::error::{IdlcError, IdlcResult};
use crate::jsonrpc::{Artifact, ArtifactFile, Codegen, CodegenInput, RpcError};
use crate::units::UnitGraph;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::sync::{Arc, Mutex};
use xidl_parser::hir::ParserProperties;

#[derive(Default)]
struct SessionCounts {
    spawns: HashMap<String, usize>,
    property_queries: HashMap<String, usize>,
    generates: HashMap<String, usize>,
}

struct FakeCodegen {
    lang: String,
    scope: Option<&'static str>,
    counts: Arc<Mutex<SessionCounts>>,
}

impl Codegen for FakeCodegen {
    fn get_engine_version(&self) -> Result<String, RpcError> {
        Ok("*".into())
    }

    fn get_properties(&self) -> Result<ParserProperties, RpcError> {
        let mut counts = self
            .counts
            .lock()
            .map_err(|err| RpcError::new(format!("test session counter poisoned: {err}")))?;
        increment(&mut counts.property_queries, &self.lang);
        let mut properties = ParserProperties::new();
        if let Some(scope) = self.scope {
            properties.insert("generation_scope".into(), scope.into());
        }
        Ok(properties)
    }

    fn generate(
        &self,
        _input: CodegenInput,
        path: String,
        _props: ParserProperties,
    ) -> Result<Vec<Artifact>, RpcError> {
        let mut counts = self
            .counts
            .lock()
            .map_err(|err| RpcError::new(format!("test session counter poisoned: {err}")))?;
        increment(&mut counts.generates, &self.lang);
        let path = if path.is_empty() {
            "generated.txt".to_owned()
        } else {
            path
        };
        Ok(vec![Artifact::new_file(ArtifactFile {
            path,
            content: String::new(),
        })])
    }
}

fn increment(counts: &mut HashMap<String, usize>, lang: &str) {
    *counts.entry(lang.to_owned()).or_default() += 1;
}

fn count(counts: &HashMap<String, usize>, lang: &str) -> usize {
    counts.get(lang).copied().unwrap_or_default()
}

fn fake_session(
    lang: &str,
    scope: Option<&'static str>,
    counts: Arc<Mutex<SessionCounts>>,
) -> CodegenSession {
    CodegenSession::from_codegen(Box::new(FakeCodegen {
        lang: lang.to_owned(),
        scope,
        counts,
    }))
}

fn counting_generator(
    lang: &str,
    scope: Option<&'static str>,
    counts: Arc<Mutex<SessionCounts>>,
) -> Generator {
    let target = lang.to_owned();
    Generator::with_session_factory(
        lang.to_owned(),
        move |session_lang: &str| -> IdlcResult<CodegenSession> {
            {
                let mut state = counts.lock().map_err(|err| {
                    IdlcError::rpc(format!("test session counter poisoned: {err}"))
                })?;
                increment(&mut state.spawns, session_lang);
            }
            if session_lang == "hir" {
                CodegenSession::spawn(session_lang)
            } else {
                let session_scope = (session_lang == target).then_some(scope).flatten();
                Ok(fake_session(
                    session_lang,
                    session_scope,
                    Arc::clone(&counts),
                ))
            }
        },
    )
}

#[test]
fn multi_unit_generation_spawns_once_per_language() {
    let counts = Arc::new(Mutex::new(SessionCounts::default()));
    let mut generator = counting_generator("test-plugin", None, Arc::clone(&counts));
    for (name, source) in [
        ("first.idl", "struct First { long value; };"),
        ("second.idl", "struct Second { long value; };"),
    ] {
        let files = generator
            .generate_from_idl(source, Path::new(name), HashMap::new())
            .expect("generate a unit");
        assert!(!files.is_empty());
    }
    let counts = counts.lock().expect("session counts");
    assert_eq!(count(&counts.spawns, "test-plugin"), 1);
    assert_eq!(count(&counts.property_queries, "test-plugin"), 1);
    assert_eq!(count(&counts.generates, "test-plugin"), 2);
    assert_eq!(count(&counts.spawns, "hir"), 0);
}

#[test]
fn compilation_scope_reuses_the_property_probe_session() {
    let counts = Arc::new(Mutex::new(SessionCounts::default()));
    let mut generator = counting_generator("docs", Some("compilation"), Arc::clone(&counts));
    let props = generator
        .get_properties_for_lang()
        .expect("probe compilation properties");
    assert_eq!(
        props
            .get("generation_scope")
            .and_then(|value| value.as_str()),
        Some("compilation")
    );

    let root = tempfile::tempdir().expect("temporary compilation root");
    let first = root.path().join("first.idl");
    let second = root.path().join("second.idl");
    std::fs::write(&first, "struct First { long value; };").expect("write first unit");
    std::fs::write(&second, "struct Second { long value; };").expect("write second unit");
    let roots = vec![first, second];
    let graph = UnitGraph::build(&roots, &BTreeMap::new()).expect("build unit graph");
    let files = generator
        .generate_compilation(&graph, props, &roots)
        .expect("generate compilation");
    assert!(!files.is_empty());

    let counts = counts.lock().expect("session counts");
    assert_eq!(count(&counts.spawns, "docs"), 1);
    assert_eq!(count(&counts.property_queries, "docs"), 1);
    assert_eq!(count(&counts.generates, "docs"), 1);
}

#[test]
fn session_pool_reuses_sessions_for_aliased_language_names() {
    let counts = Arc::new(Mutex::new(SessionCounts::default()));
    let pool_counts = Arc::clone(&counts);
    let mut pool = SessionPool::with_factory(move |lang: &str| {
        {
            let mut state = pool_counts
                .lock()
                .map_err(|err| IdlcError::rpc(format!("test session counter poisoned: {err}")))?;
            increment(&mut state.spawns, lang);
        }
        Ok(fake_session(lang, None, Arc::clone(&pool_counts)))
    });

    let first = pool.properties("go").expect("query canonical properties");
    let second = pool.properties("golang").expect("query aliased properties");
    assert_eq!(first, second);

    let counts = counts.lock().expect("session counts");
    assert_eq!(count(&counts.spawns, "go"), 1);
    assert_eq!(count(&counts.property_queries, "go"), 1);
}
const BROKEN: &str = "struct Broken {";

fn assert_syntax_diagnostics(error: IdlcError, filename: &str) {
    let IdlcError::Diagnostics(diagnostics) = error else {
        panic!("expected rich syntax diagnostics, got {error}");
    };
    assert!(!diagnostics.diagnostics.is_empty());
    assert!(
        diagnostics
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.filename.ends_with(filename))
    );
}

#[test]
fn driver_returns_structured_syntax_diagnostics_from_the_unit_parse() {
    let dir = tempfile::tempdir().expect("driver fixture");
    let input = dir.path().join("api.idl");
    std::fs::write(&input, BROKEN).expect("write invalid source");

    let Err(error) = Driver::run(ArgsGenerate {
        lang: "rust".into(),
        out_dir: "unused".into(),
        client: false,
        server: false,
        dry_run: true,
        mock: false,
        api_version: None,
        api_versions: None,
        openapi_version: None,
        files: vec![input],
    }) else {
        panic!("invalid syntax must fail");
    };

    assert_syntax_diagnostics(error, "api.idl");
}

#[test]
fn generator_returns_structured_syntax_diagnostics_from_its_single_parse() {
    let Err(error) = Generator::new("rust".into()).generate_from_idl(
        BROKEN,
        Path::new("broken.idl"),
        HashMap::new(),
    ) else {
        panic!("invalid syntax must fail");
    };

    assert_syntax_diagnostics(error, "broken.idl");
}
