mod testcases;

use super::generate_for_lang;
use std::path::Path;

fn render_lang_output(lang: &str, input_name: &str, source: &str) -> String {
    let mut files = generate_for_lang(lang, source, Path::new(input_name)).expect("generate");
    files.sort_by(|a, b| a.filename.cmp(&b.filename));

    let mut out = String::new();
    for file in files {
        out.push_str("===============\n");
        out.push_str(&file.filename);
        out.push_str("\n===============\n");
        out.push_str(&file.filecontent);
        if !file.filecontent.ends_with('\n') {
            out.push('\n');
        }
    }
    out
}

fn assert_cases(lang: &str, prefix: &str, cases: &[(&str, &str)]) {
    for (name, text) in cases {
        let input_name = format!("{name}.idl");
        let output = render_lang_output(lang, &input_name, text);
        let snapshot = format!("{lang}_{prefix}__{name}");
        insta::assert_snapshot!(snapshot, output);
    }
}

#[test]
fn test_code_gen() {
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

    let langs = ["c", "cpp", "rs"];

    for case in test_case {
        for lang in langs {
            assert_cases(lang, &format!("{lang}_gen"), case);
        }
    }
}
