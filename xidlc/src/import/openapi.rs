use crate::error::IdlcResult;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub fn import_openapi(input: &Path, out_dir: &Path) -> IdlcResult<()> {
    let content = fs::read_to_string(input)?;
    let openapi: Value = serde_json::from_str(&content)
        .map_err(|err| crate::error::IdlcError::fmt(format!("failed to parse openapi: {err}")))?;

    let mut idl = String::new();

    // Generate Pragmas
    if let Some(info) = openapi.get("info") {
        if let Some(title) = info.get("title").and_then(|v| v.as_str()) {
            idl.push_str(&format!("#pragma xidlc package {}\n", title));
        }
        if let Some(version) = info.get("version").and_then(|v| v.as_str()) {
            idl.push_str(&format!("#pragma xidlc version {}\n", version));
        }
    }

    if let Some(servers) = openapi.get("servers").and_then(|v| v.as_array()) {
        if let Some(server) = servers.first() {
            if let Some(url) = server.get("url").and_then(|v| v.as_str()) {
                idl.push_str(&format!("#pragma xidlc service {} api\n", url));
            }
        }
    }
    idl.push('\n');

    let mut ctx = ImportCtx::new();
    if let Some(components) = openapi.get("components") {
        if let Some(schemas) = components.get("schemas").and_then(|v| v.as_object()) {
            for (name, schema) in schemas {
                ctx.schemas.insert(name.clone(), schema.clone());
            }
        }
        if let Some(params) = components.get("parameters").and_then(|v| v.as_object()) {
            for (name, param) in params {
                ctx.parameters.insert(name.clone(), param.clone());
            }
        }
        if let Some(bodies) = components.get("requestBodies").and_then(|v| v.as_object()) {
            for (name, body) in bodies {
                ctx.request_bodies.insert(name.clone(), body.clone());
            }
        }
    }

    // Generate Schemas
    for (name, schema) in &ctx.schemas {
        idl.push_str(&render_schema_definition(
            &to_camel_case(name),
            schema,
            &ctx,
        ));
        idl.push('\n');
    }

    // Generate Interfaces
    idl.push_str("interface Api {\n");
    if let Some(paths) = openapi.get("paths").and_then(|v| v.as_object()) {
        for (path, path_item) in paths {
            if let Some(obj) = path_item.as_object() {
                for (method, operation) in obj {
                    match method.as_str() {
                        "get" | "post" | "put" | "delete" | "patch" => {
                            idl.push_str(&render_operation(method, path, operation, &ctx));
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    idl.push_str("};\n");

    let stem = input.file_stem().unwrap().to_str().unwrap();
    let out_path = out_dir.join(format!("{stem}.idl"));
    fs::write(out_path, idl)?;

    Ok(())
}

struct ImportCtx {
    schemas: HashMap<String, Value>,
    parameters: HashMap<String, Value>,
    request_bodies: HashMap<String, Value>,
}

impl ImportCtx {
    fn new() -> Self {
        Self {
            schemas: HashMap::new(),
            parameters: HashMap::new(),
            request_bodies: HashMap::new(),
        }
    }

    fn resolve_param<'a>(&'a self, param: &'a Value) -> &'a Value {
        if let Some(ref_val) = param.get("$ref").and_then(|v| v.as_str()) {
            let name = ref_val.split('/').last().unwrap();
            return self.parameters.get(name).unwrap_or(param);
        }
        param
    }

    fn resolve_body<'a>(&'a self, body: &'a Value) -> &'a Value {
        if let Some(ref_val) = body.get("$ref").and_then(|v| v.as_str()) {
            let name = ref_val.split('/').last().unwrap();
            return self.request_bodies.get(name).unwrap_or(body);
        }
        body
    }
}

fn render_schema_definition(name: &str, schema: &Value, ctx: &ImportCtx) -> String {
    let mut out = String::new();

    if let Some(ref_val) = schema.get("$ref").and_then(|v| v.as_str()) {
        out.push_str(&format!(
            "typedef {} {};\n",
            resolve_ref_name(ref_val),
            name
        ));
    } else if let Some(obj) = schema.as_object() {
        let ty = obj.get("type").and_then(|v| v.as_str()).unwrap_or("object");
        if ty == "object" {
            if let Some(props) = obj.get("properties").and_then(|v| v.as_object()) {
                out.push_str(&format!("struct {} {{\n", name));
                let required = obj.get("required").and_then(|v| v.as_array());
                for (prop_name, prop_schema) in props {
                    let is_required = required
                        .map(|r| r.iter().any(|v| v.as_str() == Some(prop_name)))
                        .unwrap_or(false);
                    if !is_required {
                        out.push_str("    @optional\n");
                    }
                    let fixed = fix_ident(prop_name);
                    if prop_name != &fixed {
                        out.push_str(&format!("    @rename(\"{}\")\n", prop_name));
                    }
                    out.push_str(&format!(
                        "    {} {};\n",
                        render_type(prop_schema, ctx),
                        fixed
                    ));
                }
                out.push_str("};\n");
            } else if let Some(additional) = obj.get("additionalProperties") {
                out.push_str(&format!(
                    "typedef map<string, {}> {};\n",
                    render_type(additional, ctx),
                    name
                ));
            } else {
                out.push_str(&format!("typedef string {};\n", name)); // Fallback
            }
        } else {
            out.push_str(&format!("typedef {} {};\n", render_type(schema, ctx), name));
        }
    }
    out
}

fn render_type(schema: &Value, ctx: &ImportCtx) -> String {
    if let Some(ref_val) = schema.get("$ref").and_then(|v| v.as_str()) {
        return resolve_ref_name(ref_val);
    }

    if let Some(obj) = schema.as_object() {
        if let Some(ty) = obj.get("type").and_then(|v| v.as_str()) {
            match ty {
                "string" => return "string".to_string(),
                "integer" => return "int64".to_string(),
                "number" => return "double".to_string(),
                "boolean" => return "boolean".to_string(),
                "array" => {
                    if let Some(items) = obj.get("items") {
                        return format!("sequence<{}>", render_type(items, ctx));
                    }
                    return "sequence<string>".to_string();
                }
                "object" => {
                    if let Some(additional) = obj.get("additionalProperties") {
                        return format!("map<string, {}>", render_type(additional, ctx));
                    }
                }
                _ => {}
            }
        }
    }
    "string".to_string() // Fallback
}

fn resolve_ref_name(ref_val: &str) -> String {
    to_camel_case(ref_val.split('/').last().unwrap())
}

fn render_operation(method: &str, path: &str, op: &Value, ctx: &ImportCtx) -> String {
    let mut out = String::new();
    out.push_str(&format!("    @{}(path = \"{}\")\n", method, path));

    if let Some(id) = op.get("operationId").and_then(|v| v.as_str()) {
        out.push_str(&format!("    void {}(\n", fix_ident(id)));
    } else {
        out.push_str(&format!(
            "    void op_{}(\n",
            fix_ident(&path.replace('/', "_"))
        ));
    }

    // Parameters
    let mut params_list = Vec::new();
    let mut seen_params = std::collections::HashSet::new();

    if let Some(params) = op.get("parameters").and_then(|v| v.as_array()) {
        for param in params {
            let p_resolved = ctx.resolve_param(param);
            if let Some(p) = p_resolved.as_object() {
                let name = p.get("name").and_then(|v| v.as_str()).unwrap_or("param");
                let location = p.get("in").and_then(|v| v.as_str()).unwrap_or("query");
                let required = p.get("required").and_then(|v| v.as_bool()).unwrap_or(false);
                let schema = p.get("schema").unwrap_or(&Value::Null);

                let mut fixed_name = fix_ident(name);
                // Extra escape for internal conflicts
                if fixed_name == "path"
                    || fixed_name == "query"
                    || fixed_name == "headers"
                    || fixed_name == "body"
                {
                    fixed_name = format!("arg_{}", fixed_name);
                }

                if seen_params.contains(&fixed_name) {
                    let mut i = 1;
                    while seen_params.contains(&format!("{}_{}", fixed_name, i)) {
                        i += 1;
                    }
                    fixed_name = format!("{}_{}", fixed_name, i);
                }
                seen_params.insert(fixed_name.clone());

                let mut p_str = format!("        @{}", location);
                if location == "path" {
                } else if !required {
                    p_str.push_str(" @optional");
                }
                if name != fixed_name {
                    p_str.push_str(&format!(" @rename(\"{}\")", name));
                }
                p_str.push_str(&format!(" {} {}", render_type(schema, ctx), fixed_name));
                params_list.push(p_str);
            }
        }
    }

    // Body
    if let Some(body_ref) = op.get("requestBody") {
        let body = ctx.resolve_body(body_ref);
        if let Some(content) = body.get("content").and_then(|v| v.as_object()) {
            if let Some(json_content) = content.get("application/json") {
                if let Some(schema) = json_content.get("schema") {
                    params_list.push(format!("        @body {} body", render_type(schema, ctx)));
                }
            }
        }
    }

    out.push_str(&params_list.join(",\n"));
    out.push_str("\n    );\n");
    out
}

fn fix_ident(s: &str) -> String {
    let s = s
        .replace('-', "_")
        .replace('.', "_")
        .replace(' ', "_")
        .replace('/', "_")
        .replace('{', "")
        .replace('}', "")
        .replace('+', "_")
        .replace(':', "_");
    if s.is_empty() {
        return "empty".to_string();
    }
    // Handle Rust keywords by adding _ suffix
    match s.as_str() {
        "as" | "break" | "const" | "continue" | "crate" | "else" | "enum" | "extern" | "false"
        | "fn" | "for" | "if" | "impl" | "in" | "let" | "loop" | "match" | "mod" | "move"
        | "mut" | "pub" | "ref" | "return" | "self" | "Self" | "static" | "struct" | "super"
        | "trait" | "true" | "type" | "unsafe" | "use" | "where" | "while" | "async" | "await"
        | "dyn" | "abstract" | "become" | "box" | "do" | "final" | "macro" | "override"
        | "priv" | "typeof" | "unsized" | "virtual" | "yield" | "try" => {
            return format!("{}_", s);
        }
        _ => {}
    }
    // Ensure it doesn't start with a number
    if s.chars().next().unwrap().is_ascii_digit() {
        return format!("_{}", s);
    }
    s
}

fn to_camel_case(s: &str) -> String {
    let s = s
        .replace('-', "_")
        .replace('.', "_")
        .replace(' ', "_")
        .replace('/', "_")
        .replace('{', "")
        .replace('}', "")
        .replace('+', "_")
        .replace(':', "_");
    let mut out = String::new();
    let mut capitalize = true;
    for c in s.chars() {
        if c == '_' {
            capitalize = true;
        } else if capitalize {
            out.push(c.to_ascii_uppercase());
            capitalize = false;
        } else {
            out.push(c);
        }
    }
    // Still check for keywords after camel casing (though unlikely)
    fix_ident(&out)
}
