use minijinja::Environment;
use serde_json::Value;

/// Renders the OpenAPI document as a JSON string.
pub fn render_openapi_json_string(value: &Value) -> Result<String, minijinja::Error> {
    let mut env = Environment::new();
    env.add_template("openapi.json", "{{ doc | tojson(pretty=true) }}")?;
    let tmpl = env.get_template("openapi.json")?;
    tmpl.render(serde_json::json!({ "doc": value }))
}
