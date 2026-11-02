use crate::error::Result;
use crate::markdown::Markdown;
use minijinja::Environment;
use serde::Serialize;

pub(crate) struct Renderer {
    environment: Environment<'static>,
}

impl Renderer {
    pub fn new() -> Result<Self> {
        let mut environment = Environment::new();
        for (name, source) in [
            ("operation.md", include_str!("templates/operation.md.j2")),
            ("type.md", include_str!("templates/type.md.j2")),
            ("changes.md", include_str!("templates/changes.md.j2")),
            ("index.md", include_str!("templates/index.md.j2")),
            ("llms.txt", include_str!("templates/llms.txt.j2")),
            ("starlight.md", include_str!("templates/starlight.md.j2")),
        ] {
            environment.add_template(name, source)?;
        }
        environment.add_filter("inline", Self::inline);
        environment.add_filter("markdown", Markdown::safe_fragment);
        Ok(Self { environment })
    }

    pub fn template(&self, name: &str, context: impl Serialize) -> Result<String> {
        Ok(self.environment.get_template(name)?.render(context)?)
    }

    fn inline(value: String) -> String {
        value
            .replace('\\', "\\\\")
            .replace('[', "\\[")
            .replace(']', "\\]")
            .replace('*', "\\*")
            .replace('_', "\\_")
            .replace('`', "\\`")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('|', "\\|")
            .replace(['\n', '\r'], " ")
    }
}
