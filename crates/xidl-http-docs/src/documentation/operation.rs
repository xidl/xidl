use super::{MethodDocumentation, metadata::Metadata};
use crate::error::Result;
use serde_json::Value;
use xidl_parser::hir;
use xidl_parser::rest_hir::{
    HttpOperation, HttpOutputSource, HttpRequestBodyShape, HttpResponseBodyShape,
};

pub(super) struct OperationDocumentation<'a> {
    pub source: Option<&'a MethodDocumentation>,
    pub operation: &'a HttpOperation,
}

struct FieldMetadata<'a> {
    name: &'a str,
    ty: &'a hir::TypeSpec,
    source: Option<&'a Metadata>,
}

impl OperationDocumentation<'_> {
    pub fn enrich(&self, value: &mut Value) -> Result<()> {
        if let Some(deprecated) = &self.operation.meta.deprecated {
            value["x-xidl-deprecation"] = serde_json::to_value(deprecated)?;
        }
        for parameter in value
            .get_mut("parameters")
            .and_then(Value::as_array_mut)
            .into_iter()
            .flatten()
        {
            let bindings = match parameter["in"].as_str() {
                Some("path") => &self.operation.http.request.path,
                Some("query") => &self.operation.http.request.query,
                Some("header") => &self.operation.http.request.header,
                Some("cookie") => &self.operation.http.request.cookie,
                _ => continue,
            };
            if let Some(binding) = bindings
                .iter()
                .find(|binding| parameter["name"].as_str() == Some(&binding.wire_name))
            {
                if let Some(metadata) = self.parameter(&binding.source_param) {
                    metadata.apply(&mut parameter["schema"])?;
                    for key in ["description", "deprecated", "x-xidl-deprecation"] {
                        if let Some(value) = parameter["schema"].get(key).cloned() {
                            parameter[key] = value;
                        }
                    }
                }
                Metadata::bounds(&binding.ty, &mut parameter["schema"]);
            }
        }
        for content in value
            .get_mut("requestBody")
            .and_then(|body| body.get_mut("content"))
            .and_then(Value::as_object_mut)
            .into_iter()
            .flat_map(|content| content.values_mut())
        {
            let Some(schema) = content.get_mut("schema") else {
                continue;
            };
            match &self.operation.http.request.body.shape {
                HttpRequestBodyShape::SingleValue {
                    source_param, ty, ..
                } => {
                    if let Some(metadata) = self.parameter(source_param) {
                        metadata.apply(schema)?;
                    }
                    Metadata::bounds(ty, schema);
                }
                HttpRequestBodyShape::Object { fields } => {
                    for field in fields {
                        Self::field(
                            schema,
                            &FieldMetadata {
                                name: &field.field_name,
                                ty: &field.ty,
                                source: self.parameter(&field.source_param),
                            },
                        )?;
                    }
                }
                _ => {}
            }
        }
        if let Some(response) = value
            .get_mut("responses")
            .and_then(|responses| responses.get_mut(&self.operation.http.response.status))
        {
            self.response(response)?;
        }
        Ok(())
    }

    fn parameter(&self, name: &str) -> Option<&Metadata> {
        self.source.and_then(|source| source.parameters.get(name))
    }

    fn field(schema: &mut Value, metadata: &FieldMetadata<'_>) -> Result<()> {
        if let Some(field) = schema
            .get_mut("properties")
            .and_then(|properties| properties.get_mut(metadata.name))
        {
            if let Some(source) = metadata.source {
                source.apply(field)?;
            }
            Metadata::bounds(metadata.ty, field);
        }
        for part in schema
            .get_mut("allOf")
            .and_then(Value::as_array_mut)
            .into_iter()
            .flatten()
        {
            Self::field(part, metadata)?;
        }
        Ok(())
    }

    fn output(&self, source: &HttpOutputSource) -> Option<&Metadata> {
        match source {
            HttpOutputSource::Param { name } => self.parameter(name),
            HttpOutputSource::ReturnValue => None,
        }
    }

    fn response(&self, response: &mut Value) -> Result<()> {
        for content in response
            .get_mut("content")
            .and_then(Value::as_object_mut)
            .into_iter()
            .flat_map(|content| content.values_mut())
        {
            let Some(schema) = content.get_mut("schema") else {
                continue;
            };
            match &self.operation.http.response.body.shape {
                HttpResponseBodyShape::ReturnOnly { ty }
                | HttpResponseBodyShape::SingleValue { ty, .. } => Metadata::bounds(ty, schema),
                HttpResponseBodyShape::Object { fields } => {
                    for field in fields {
                        Self::field(
                            schema,
                            &FieldMetadata {
                                name: &field.field_name,
                                ty: &field.ty,
                                source: self.output(&field.source),
                            },
                        )?;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}
