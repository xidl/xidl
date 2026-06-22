use convert_case::{Case, Casing};

pub(crate) fn go_export_name(prefix: &[String], value: &str) -> String {
    let mut parts = prefix
        .iter()
        .map(|part| part.to_case(Case::Pascal))
        .collect::<Vec<_>>();
    parts.push(go_capitalize(value));
    parts.join("")
}

pub(crate) fn go_capitalize(value: &str) -> String {
    if value.contains('_') {
        value.to_case(Case::Pascal)
    } else {
        let mut chars = value.chars();
        match chars.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
        }
    }
}

pub(crate) fn go_field_name(value: &str) -> String {
    let ident = go_capitalize(value);
    if go_keyword(&ident) {
        format!("{ident}_")
    } else {
        ident
    }
}

pub(crate) fn pointer_type(ty: &str) -> String {
    if ty.starts_with('*') {
        ty.to_string()
    } else {
        format!("*{ty}")
    }
}

fn go_keyword(value: &str) -> bool {
    matches!(
        value,
        "Break"
            | "Default"
            | "Func"
            | "Interface"
            | "Select"
            | "Case"
            | "Defer"
            | "Go"
            | "Map"
            | "Struct"
            | "Chan"
            | "Else"
            | "Goto"
            | "Package"
            | "Switch"
            | "Const"
            | "Fallthrough"
            | "If"
            | "Range"
            | "Type"
            | "Continue"
            | "For"
            | "Import"
            | "Return"
            | "Var"
    )
}
