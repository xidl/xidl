use std::collections::HashMap;
use std::fs;
use std::path::Path;

use minijinja::Environment;
use regex::Regex;
use serde::Deserialize;

use similar::{ChangeTag, TextDiff};
use xidlc_examples::http_server::{HttpServerServer, SimpleHttpServer};

#[derive(Debug, Clone)]
struct HttpTest {
    name: String,
    method: String,
    url: String,
    headers: Vec<(String, String)>,
    body: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct SnapshotConfig {
    #[serde(default)]
    drop_headers: Vec<String>,
    #[serde(default)]
    body_filters: Vec<BodyFilter>,
}

#[derive(Debug, Default, Deserialize)]
struct BodyFilter {
    pattern: String,
    #[serde(default)]
    replace: String,
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_snapshot_tests() {
    let listener = match std::net::TcpListener::bind("127.0.0.1:0") {
        Ok(listener) => listener,
        Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => return,
        Err(err) => panic!("bind ephemeral port: {err}"),
    };
    let addr = listener.local_addr().expect("read local addr");
    listener
        .set_nonblocking(true)
        .expect("set listener nonblocking");
    let listener = tokio::net::TcpListener::from_std(listener).expect("adopt listener for tokio");

    let task = tokio::spawn(async move {
        xidl_rust_axum::Server::builder()
            .with_service(HttpServerServer::new(SimpleHttpServer::new()))
            .serve_with_listener(listener)
            .await
    });

    tokio::time::sleep(std::time::Duration::from_millis(150)).await;

    let mut vars = serde_json::Map::new();
    vars.insert("base_url".to_string(), format!("http://{addr}").into());

    run_http_snapshots(&vars).await;

    task.abort();
}

async fn run_http_snapshots(vars: &serde_json::Map<String, serde_json::Value>) {
    let defs_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/http_snapshots/defs");
    let out_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/http_snapshots/snapshots");
    fs::create_dir_all(&out_root).expect("create snapshot output dir");

    let mut entries = fs::read_dir(&defs_root).expect("read http snapshot defs");
    let mut files = Vec::new();
    while let Some(entry) = entries.next() {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("http") {
            files.push(path);
        }
    }
    files.sort();
    assert!(!files.is_empty(), "no http snapshot definitions found");

    for def_path in files {
        let rendered = render_definition(&def_path, vars).expect("render definition");
        let config = load_snapshot_config(&def_path).expect("load snapshot config");
        let tests = parse_definition(&rendered).expect("parse definition");
        let output = run_tests(&tests, &config).await;

        let snapshot_name = def_path
            .file_stem()
            .and_then(|value| value.to_str())
            .expect("definition stem");
        let snapshot_path = out_root.join(format!("{snapshot_name}.snap"));
        assert_snapshot(&snapshot_path, &output);
    }
}

fn render_definition(
    path: &Path,
    vars: &serde_json::Map<String, serde_json::Value>,
) -> anyhow::Result<String> {
    let source = fs::read_to_string(path)?;
    let env = Environment::new();
    let env_vars = std::env::vars().collect::<HashMap<_, _>>();
    let mut context = serde_json::Map::new();
    for (key, value) in vars {
        context.insert(key.clone(), value.clone());
    }
    context.insert("env".to_string(), serde_json::to_value(env_vars)?);
    env.render_str(&source, &context)
        .map_err(|err| anyhow::anyhow!(err.to_string()))
}

fn load_snapshot_config(path: &Path) -> anyhow::Result<SnapshotConfig> {
    let config_path = path.with_extension("snap.json");
    if !config_path.exists() {
        return Ok(SnapshotConfig::default());
    }
    let raw = fs::read_to_string(config_path)?;
    let mut config: SnapshotConfig = serde_json::from_str(&raw)?;
    config.drop_headers.extend(["date".to_string()].into_iter());
    Ok(config)
}

fn parse_definition(source: &str) -> anyhow::Result<Vec<HttpTest>> {
    let mut tests = Vec::new();
    let mut current: Option<HttpTest> = None;
    let mut in_body = false;
    let mut body_lines: Vec<String> = Vec::new();
    let mut test_name = String::default();

    for raw_line in source.lines() {
        let line = raw_line.trim_end();
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if current.is_some() && !in_body {
                in_body = true;
            } else if in_body {
                body_lines.push(String::new());
            }
            continue;
        }
        if trimmed.starts_with("###") {
            test_name = trimmed.to_string();
            flush_current(&mut tests, &mut current, &mut body_lines);
            in_body = false;
            continue;
        }
        if trimmed.starts_with('#') {
            continue;
        }
        if is_method_line(trimmed) {
            flush_current(&mut tests, &mut current, &mut body_lines);
            let (method, url) = split_method_line(trimmed)?;
            current = Some(HttpTest {
                name: test_name.clone(),
                method,
                url,
                headers: Vec::new(),
                body: None,
            });
            in_body = false;
            continue;
        }

        let Some(test) = current.as_mut() else {
            continue;
        };
        if !in_body && trimmed.contains(':') {
            let (name, value) = trimmed
                .split_once(':')
                .ok_or_else(|| anyhow::anyhow!("invalid header line: {trimmed}"))?;
            test.headers
                .push((name.trim().to_string(), value.trim().to_string()));
        } else {
            in_body = true;
            body_lines.push(line.to_string());
        }
    }

    flush_current(&mut tests, &mut current, &mut body_lines);

    if tests.is_empty() {
        return Err(anyhow::anyhow!("no http tests defined"));
    }
    Ok(tests)
}

fn flush_current(
    tests: &mut Vec<HttpTest>,
    current: &mut Option<HttpTest>,
    body_lines: &mut Vec<String>,
) {
    if let Some(mut test) = current.take() {
        if !body_lines.is_empty() {
            let body = body_lines.join("\n");
            if !body.trim().is_empty() {
                test.body = Some(body);
            }
        }
        tests.push(test);
    }
    body_lines.clear();
}

fn is_method_line(line: &str) -> bool {
    let method = line.split_whitespace().next().unwrap_or("");
    matches!(
        method,
        "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD" | "OPTIONS"
    )
}

fn split_method_line(line: &str) -> anyhow::Result<(String, String)> {
    let mut parts = line.split_whitespace();
    let method = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing method"))?
        .to_string();
    let url = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing url"))?
        .to_string();
    Ok((method, url))
}

async fn run_tests(tests: &[HttpTest], config: &SnapshotConfig) -> String {
    let client = reqwest::Client::builder()
        .build()
        .expect("build reqwest client without proxy");
    let mut sections = Vec::new();
    for test in tests {
        let section = run_test(&client, test, config).await;
        sections.push(section);
    }
    sections.join("\n---\n")
}

async fn run_test(client: &reqwest::Client, test: &HttpTest, config: &SnapshotConfig) -> String {
    let url = reqwest::Url::parse(&test.url)
        .unwrap_or_else(|_| panic!("invalid url in http snapshot: {}", test.url));
    let path = if url.query().is_some() {
        format!("{}?{}", url.path(), url.query().unwrap())
    } else {
        url.path().to_string()
    };

    let mut request_lines = Vec::new();
    let display_name = if test.name.trim().is_empty() {
        format!("{} {path}", test.method)
    } else {
        test.name.clone()
    };
    request_lines.push(display_name);
    request_lines.push(format!("> {} {path} HTTP/1.1", test.method));
    request_lines.push("> User-Agent: xidlc-http-snapshot".to_string());
    for (name, value) in &test.headers {
        request_lines.push(format!("> {name}: {value}"));
    }
    request_lines.push(">".to_string());
    if let Some(body) = &test.body {
        request_lines.push(body.clone());
    }

    let mut req = client.request(test.method.parse().unwrap(), url.clone());
    req = req.header("User-Agent", "xidlc-http-snapshot");
    for (name, value) in &test.headers {
        req = req.header(name, value);
    }
    if let Some(body) = &test.body {
        req = req.body(body.clone());
    }
    let resp = req.send().await.expect("send request");
    let status = resp.status();
    let reason = status.canonical_reason().unwrap_or("");
    let mut response_lines = Vec::new();
    response_lines.push(format!("< HTTP/1.1 {} {}", status.as_u16(), reason));

    let mut headers = Vec::new();
    let drop_headers = normalized_drop_headers(config);
    for (name, value) in resp.headers().iter() {
        let name_str = name.as_str().to_ascii_lowercase();
        if drop_headers.iter().any(|value| value == &name_str) {
            continue;
        }
        let value_str = value.to_str().unwrap_or("").to_string();
        headers.push((name_str, value_str));
    }
    headers.sort_by(|a, b| a.0.cmp(&b.0));
    for (name, value) in headers {
        response_lines.push(format!("< {name}: {value}"));
    }
    response_lines.push("<".to_string());

    let mut body = resp.text().await.unwrap_or_default();
    body = apply_body_filters(body, config);
    if !body.is_empty() {
        response_lines.push(body);
    }

    request_lines
        .into_iter()
        .chain(response_lines)
        .collect::<Vec<_>>()
        .join("\n")
}

fn normalized_drop_headers(config: &SnapshotConfig) -> Vec<String> {
    let mut headers = vec!["date".to_string()];
    headers.extend(config.drop_headers.iter().cloned());
    headers
        .into_iter()
        .map(|value| value.to_ascii_lowercase())
        .collect()
}

fn apply_body_filters(mut body: String, config: &SnapshotConfig) -> String {
    for filter in &config.body_filters {
        if let Ok(regex) = Regex::new(&filter.pattern) {
            body = regex
                .replace_all(&body, filter.replace.as_str())
                .to_string();
        }
    }
    body
}

fn assert_snapshot(snapshot_path: &Path, output: &str) {
    let update = std::env::var("UPDATE_HTTP_SNAPSHOTS")
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    if update {
        fs::write(snapshot_path, output).expect("write snapshot");
        return;
    }
    let expected = fs::read_to_string(snapshot_path)
        .expect("snapshot missing; set UPDATE_HTTP_SNAPSHOTS=1 to create");
    if expected != output {
        let output = output.to_string();
        let diff = TextDiff::from_lines(&expected, &output);

        eprintln!("snapshot mismatch for {snapshot_path:?}");
        for change in diff.iter_all_changes() {
            let sign = match change.tag() {
                ChangeTag::Delete => "-",
                ChangeTag::Insert => "+",
                ChangeTag::Equal => " ",
            };
            print!("{}{}", sign, change);
        }
        panic!("")
    }
}
