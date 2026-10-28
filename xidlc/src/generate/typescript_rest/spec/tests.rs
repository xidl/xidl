use super::{TsHttpOutput, format_output};

fn output_with(source: &str) -> TsHttpOutput {
    TsHttpOutput {
        types: source.to_string(),
        zod: source.to_string(),
        client: source.to_string(),
        server: source.to_string(),
        errors: source.to_string(),
    }
}

#[test]
fn formats_every_rendered_typescript_artifact() {
    let output = format_output(output_with("const x={a:1};\n")).expect("format output");
    let expected = "const x = { a: 1 };\n";
    assert_eq!(output.types, expected);
    assert_eq!(output.zod, expected);
    assert_eq!(output.client, expected);
    assert_eq!(output.server, expected);
    assert_eq!(output.errors, expected);
}

#[test]
fn keeps_unparseable_artifacts_untouched() {
    let source = "const x =\n";
    let output = format_output(output_with(source)).expect("format output");
    assert_eq!(output.types, source);
    assert_eq!(output.zod, source);
    assert_eq!(output.client, source);
    assert_eq!(output.server, source);
    assert_eq!(output.errors, source);
}

#[test]
fn keeps_empty_artifacts_untouched() {
    let output = format_output(output_with(" \n\t")).expect("format output");
    assert_eq!(output.types, " \n\t");
}
