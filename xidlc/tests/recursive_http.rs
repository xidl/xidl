#![cfg(feature = "gen-rust-axum")]

use std::{collections::HashMap, path::Path};

#[test]
fn recursive_http_models_preserve_owned_layout_in_both_wire_directions() {
    let source = include_str!("shared/recursive_http_graph.idl");
    let files = xidlc::driver::Generator::new("rust-axum".into())
        .generate_from_idl(
            source,
            Path::new("recursive_graph.idl"),
            HashMap::from([
                ("enable_client".into(), true.into()),
                ("enable_server".into(), true.into()),
            ]),
        )
        .expect("legal recursive HTTP graph");
    assert_eq!(files.len(), 1);
    let output = files[0].content();
    // Containers supply their own indirection; direct cycles need a Box.
    assert!(output.contains("pub previous: Option<Box<Node>>"));
    assert!(output.contains("pub children: Option<Vec<Node>>"));
    assert!(output.contains("pub pair: Option<Box<[Node; 2]>>"));
    for direction in ["in", "out"] {
        let module = format!("__xidl_{direction}_Graphs");
        assert!(output.contains(&format!("pub previous: Option<Box<{module}::graph_Node>>")));
        assert!(output.contains(&format!("pub pair: Option<Box<[{module}::graph_Node; 2]>>")));
    }
    insta::assert_snapshot!("owned_recursive_http_graph", output);
}
