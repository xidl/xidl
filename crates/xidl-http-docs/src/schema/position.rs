/// Distinguishes contract keywords, user-defined names and literal JSON payloads.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SchemaPosition {
    /// An OpenAPI or JSON Schema object.
    Node,
    /// An object whose keys are user-defined names.
    Names,
    /// An example, default or other literal value.
    Literal,
}

impl SchemaPosition {
    /// Classifies a child without interpreting user-defined names as keywords.
    pub fn child(self, key: &str) -> Self {
        match self {
            Self::Literal => Self::Literal,
            Self::Names => Self::Node,
            Self::Node => match key {
                "example" | "examples" | "enum" | "default" | "const" => Self::Literal,
                "properties" | "patternProperties" | "$defs" | "definitions" | "schemas"
                | "headers" | "responses" | "content" | "securitySchemes" | "encoding"
                | "dependentSchemas" => Self::Names,
                _ => Self::Node,
            },
        }
    }
}
