mod declarations;
mod history;
mod imports;

use crate::hir::{ParserProperties, Specification};
use crate::rest_hir::ProjectedHir;
use serde_json::Value;
use std::collections::BTreeMap;

fn project(source: &str, properties: ParserProperties) -> Specification {
    let typed = crate::parser::parser_text(source).expect("constant IDL syntax");
    let mut projections =
        Specification::project_typed_ast_versions(typed, properties, "constants.idl")
            .expect("project integer constant contract");
    assert_eq!(projections.len(), 1);
    let ProjectedHir::Rpc(spec) = projections.remove(0).hir else {
        panic!("integer constant RPC projection");
    };
    spec
}

fn values(spec: &Specification) -> BTreeMap<String, Value> {
    fn collect(value: &Value, output: &mut BTreeMap<String, Value>) {
        match value {
            Value::Object(object) => {
                if let Some(constant) = object.get("ConstDcl") {
                    output.insert(
                        constant["ident"].as_str().expect("constant name").into(),
                        constant["value"].clone(),
                    );
                }
                for value in object.values() {
                    collect(value, output);
                }
            }
            Value::Array(array) => {
                for value in array {
                    collect(value, output);
                }
            }
            _ => {}
        }
    }
    let mut output = BTreeMap::new();
    collect(
        &serde_json::to_value(spec).expect("constant HIR"),
        &mut output,
    );
    output
}

fn integer(value: i128) -> Value {
    let literal = serde_json::json!({"Literal": {"IntegerLiteral": value.abs().to_string()}});
    if value < 0 {
        serde_json::json!({"UnaryExpr": ["Sub", literal]})
    } else {
        literal
    }
}
