use super::{Diagnostic, DiagnosticBag, DiagnosticCode, Severity, SyntaxPass};

#[test]
fn syntax_pass_reports_sorted_deduped_syntax_errors() {
    let valid = "struct Item { string id; };";
    assert!(SyntaxPass::check_source(valid).unwrap().is_empty());

    let broken = "struct Item { string id };";
    let diagnostics = SyntaxPass::check_source(broken).unwrap();
    assert!(!diagnostics.is_empty());
    assert!(
        diagnostics.iter().all(
            |item| item.code == DiagnosticCode::SyntaxError && item.severity == Severity::Error
        )
    );
    assert!(diagnostics.iter().all(Diagnostic::has_span));
}

#[test]
fn diagnostic_bag_tracks_errors_and_sorts_by_span() {
    let mut bag = DiagnosticBag::new();
    assert!(bag.is_empty());
    assert!(!bag.has_errors());
    assert!(bag.check_errors().is_ok());

    bag.push(
        Diagnostic::warning(DiagnosticCode::UnknownAnnotation, 20..25, "unknown")
            .with_suggestion("known"),
    );
    assert!(!bag.has_errors());
    assert!(bag.check_errors().is_ok());
    assert_eq!(bag.as_slice().len(), 1);

    bag.push(
        Diagnostic::error(DiagnosticCode::SemanticError, 0..0, "bad union")
            .with_location("MyUnion"),
    );
    assert!(bag.has_errors());
    let err = bag.check_errors().unwrap_err();
    assert!(err.to_string().contains("bad union"));

    let sorted = bag.into_sorted();
    assert_eq!(sorted[0].span, 0..0);
    assert_eq!(sorted[0].location.as_deref(), Some("MyUnion"));
    assert_eq!(sorted[1].span, 20..25);
    assert_eq!(sorted[1].suggestion.as_deref(), Some("known"));
}

#[test]
fn diagnostic_code_strings_are_stable() {
    assert_eq!(DiagnosticCode::SyntaxError.as_str(), "syntax-error");
    assert_eq!(DiagnosticCode::SemanticError.as_str(), "semantic-error");
    assert_eq!(DiagnosticCode::VersioningError.as_str(), "versioning-error");
    assert_eq!(DiagnosticCode::ProtocolError.as_str(), "protocol-error");
}

#[test]
fn diagnostic_bag_dedup_preserves_order_and_removes_duplicates() {
    let mut bag = DiagnosticBag::new();
    let first = Diagnostic::error(DiagnosticCode::SemanticError, 0..0, "dup").with_location("A");
    let second = Diagnostic::error(DiagnosticCode::ProtocolError, 0..0, "other").with_location("B");
    bag.push(first.clone());
    bag.push(second.clone());
    bag.push(first.clone());
    bag.dedup();
    assert_eq!(bag.as_slice(), &[first, second]);
}
