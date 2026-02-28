mod testcases;

mod test_rust;
mod test_rust_axum;
mod test_rust_jsonrpc;

use std::{collections::HashMap, path::Path};

use crate::{
    driver::{File, generate},
    error::IdlcResult,
};

pub async fn generate_from_idl(
    source: &str,
    path: &std::path::Path,
    lang: &str,
    props: HashMap<String, serde_json::Value>,
) -> IdlcResult<Vec<File>> {
    let mut generator = generate::Generator::new(lang.into());
    generator.generate_from_idl(source, path, props).await
}

async fn render_lang_output(lang: &str, input_name: &str, source: &str) -> String {
    let mut files = generate_from_idl(source, Path::new(input_name), lang, Default::default())
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

async fn assert_cases(lang: &str, prefix: &str, cases: &[(&str, &str)]) {
    for (name, text) in cases {
        let input_name = format!("{name}.idl");
        let output = if lang == "fmt" {
            crate::fmt::format_idl_source(text).unwrap()
        } else {
            render_lang_output(lang, &input_name, text).await
        };
        let snapshot = format!("{lang}_{prefix}__{name}");
        insta::assert_snapshot!(snapshot, output);
    }
}

#[tokio::test(flavor = "current_thread")]
async fn test_code_gen() {
    let test_case = [
        testcases::ANNOTATION_CASES,
        testcases::BITMASK_CASES,
        testcases::BITSET_CASES,
        testcases::CONST_CASES,
        testcases::ENUM_CASES,
        testcases::EXCEPT_CASES,
        testcases::INTERFACE_CASES,
        testcases::MISC_CASES,
        testcases::MODULE_CASES,
        testcases::PREPROC_CASES,
        testcases::STRUCT_CASES,
        testcases::TEMPLATE_MODULE_CASES,
        testcases::TYPEDEF_CASES,
        testcases::UNION_CASES,
    ];

    let langs = ["c", "cpp", "rs", "fmt"];

    for case in test_case {
        for lang in langs {
            assert_cases(lang, &format!("{lang}_gen"), case).await;
        }
    }
}
