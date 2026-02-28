mod cases;

use super::generate_from_idl;

use std::{collections::HashMap, path::Path};

async fn render_lang_output(
    lang: &str,
    input_name: &str,
    source: &str,
    props: HashMap<String, serde_json::Value>,
) -> String {
    let mut files = generate_from_idl(source, Path::new(input_name), lang, props)
        .await
        .expect("generate");
    files.sort_by(|a, b| a.path.cmp(&b.path));

    let mut out = String::new();
    for file in files {
        let path = file.path;
        let content = file.content;
        {
            out.push_str("===============\n");
            out.push_str(&path);
            out.push_str("\n===============\n");
            out.push_str(&content);
            if !content.ends_with('\n') {
                out.push('\n');
            }
        }
    }
    out
}

async fn assert_cases(
    lang: &str,
    prefix: &str,
    name: &str,
    idl: &str,
    props: HashMap<String, serde_json::Value>,
) {
    let input_name = format!("{name}.idl");
    let output = render_lang_output(lang, &input_name, idl, props).await;
    let snapshot = format!("{lang}_{prefix}__{name}");
    insta::assert_snapshot!(snapshot, output);
}

#[tokio::test(flavor = "current_thread")]
async fn test_code_gen() {
    let test_case = cases::get_test_cases();

    for (name, idl, attr) in test_case {
        let attr: HashMap<String, serde_json::Value> = serde_json::from_value(attr).unwrap();

        assert_cases("rust_axum", "rust_gen", name, idl, attr).await;
    }
}
