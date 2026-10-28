use super::parse;
use crate::{hir, rest_hir};

#[test]
fn named_collections_share_wire_shapes_without_erasing_signature_types() {
    let spec = parse(
        r#"
        module Filters {
            typedef string Text;
            typedef sequence<Text> Names, Tags;
            typedef Names Selection;
        };
        interface Search {
            @get(path="/search")
            void read(@query Filters::Selection values, @header Filters::Tags tags,
                      @cookie Filters::Names names, @query @optional Filters::Selection labels);
        };
        "#,
    );
    let document = rest_hir::project(&spec).expect("named collection projection");
    let operation = &document.interfaces[0].operations[0];
    let request = &operation.http.request;
    for binding in request
        .query
        .iter()
        .chain(&request.header)
        .chain(&request.cookie)
    {
        let hir::TypeSpec::SequenceType(sequence) = &binding.ty else {
            panic!("{} must encode as a sequence", binding.source_param);
        };
        assert!(matches!(*sequence.ty, hir::TypeSpec::StringType(_)));
        assert!(!binding.requires_presence(rest_hir::HttpParamKind::Query));
    }
    assert!(request.query[1].optional);
    assert!(operation.signature.params.iter().all(|parameter| {
        matches!(&parameter.ty, hir::TypeSpec::ScopedName(name)
            if name.is_root && name.name[0] == "Filters")
    }));
    assert!(!operation.signature.params[0].is_optional);
}

#[test]
fn aliases_resolve_at_their_declaration_scope_and_respect_shadowing() {
    let spec = parse(
        r#"
        module Outer {
            typedef string Item;
            typedef sequence<Item> Values;
            typedef Values Selected;
            module Inner {
                typedef long Item;
                struct Values { long value; };
                interface Search {
                    typedef sequence<Item> Local;
                    @get(path="/search")
                    void read(@query Selected selected, @query ::Outer::Values absolute,
                              @query Local local, @query Values shadowed);
                };
            };
        };
        "#,
    );
    let document = rest_hir::project(&spec).expect("scoped collection projection");
    let inputs = &document.interfaces[0].operations[0].http.request.query;
    for binding in &inputs[..2] {
        assert!(matches!(&binding.ty, hir::TypeSpec::SequenceType(sequence)
            if matches!(*sequence.ty, hir::TypeSpec::StringType(_))));
    }
    assert!(
        matches!(&inputs[2].ty, hir::TypeSpec::SequenceType(sequence)
        if matches!(*sequence.ty, hir::TypeSpec::IntegerType(_)))
    );
    assert!(matches!(&inputs[3].ty, hir::TypeSpec::ScopedName(name)
        if name.is_root && name.name == ["Outer", "Inner", "Values"]));
}

#[test]
fn body_and_stream_mappings_keep_the_same_named_references_as_the_signature() {
    let spec = parse(
        r#"
        module api {
            module filters { typedef sequence<string> Selection; };
            interface Search {
                @post(path="/selection")
                filters::Selection select(@body filters::Selection value);
                @server_stream @get(path="/selection-events")
                filters::Selection selection_events();
            };
        };
        "#,
    );
    let document = rest_hir::project(&spec).expect("body and stream alias projection");
    let operations = &document.interfaces[0].operations;
    let rest_hir::HttpRequestBodyShape::SingleValue { ty: input, .. } =
        &operations[0].http.request.body.shape
    else {
        panic!("selection body mapping")
    };
    let rest_hir::HttpResponseBodyShape::ReturnOnly { ty: output } =
        &operations[0].http.response.body.shape
    else {
        panic!("selection response mapping")
    };
    let rest_hir::HttpResponseBodyShape::Stream { item_ty: item, .. } =
        &operations[1].http.response.body.shape
    else {
        panic!("selection event mapping")
    };
    for ty in [input, output, item, &operations[0].signature.params[0].ty] {
        assert!(matches!(ty, hir::TypeSpec::ScopedName(name)
            if name.is_root && name.name == ["api", "filters", "Selection"]));
    }
}

#[test]
fn fixed_arrays_and_named_sequence_elements_keep_their_declared_identity() {
    let spec = parse(
        r#"
        module Types {
            typedef long Pair[2];
            typedef Pair Coordinates;
            enum Kind { image, video };
            typedef sequence<Kind> Kinds;
        };
        interface Search {
            @get(path="/search")
            void read(@query Types::Coordinates point, @query Types::Kinds kinds);
        };
        "#,
    );
    let document = rest_hir::project(&spec).expect("fixed array and enum projection");
    let inputs = &document.interfaces[0].operations[0].http.request.query;
    assert!(matches!(&inputs[0].ty, hir::TypeSpec::ScopedName(name)
        if name.is_root && name.name == ["Types", "Pair"]));
    assert!(inputs[0].requires_presence(rest_hir::HttpParamKind::Query));
    assert!(
        matches!(&inputs[1].ty, hir::TypeSpec::SequenceType(sequence)
        if matches!(&*sequence.ty, hir::TypeSpec::ScopedName(name)
            if name.is_root && name.name == ["Types", "Kind"]))
    );
}

#[test]
fn recursive_typedef_input_fails_with_its_operation_and_parameter() {
    let spec = parse(
        r#"
        typedef sequence<Loop> Loop;
        interface Search {
            @get(path="/search") void read(@query Loop values);
        };
        "#,
    );
    let error = rest_hir::project(&spec).expect_err("cyclic HTTP typedef");
    assert!(error.to_string().contains("Search.read::values"));
    assert!(error.to_string().contains("typedef cycle includes 'Loop'"));
}

#[test]
fn exception_members_preserve_scoped_collection_and_enum_aliases() {
    let spec = parse(
        r#"
        module Codes {
            enum State { @rename("true") blocked };
            typedef State RetryState;
            typedef sequence<long, 2> Values;
        };
        module Billing {
            @http(409) exception Rejected {
                Codes::Values values;
                @header Codes::Values retry;
                @cookie Codes::RetryState state;
            };
            interface Batches { @post void submit() raises(Rejected); };
        };
        "#,
    );
    let document = rest_hir::project(&spec).expect("scoped exception wire types");
    let exception = &document.document.exceptions[0];
    for (ty, ident) in [
        (&exception.body[0].ty, "Values"),
        (&exception.headers[0].ty, "Values"),
        (&exception.cookies[0].ty, "RetryState"),
    ] {
        assert!(matches!(ty, hir::TypeSpec::ScopedName(name)
            if name.is_root && name.name == ["Codes", ident]));
    }
    assert!(
        !exception.headers[0].is_multi,
        "aliases retain their metadata encoding"
    );
}
