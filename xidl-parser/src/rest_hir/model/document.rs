use super::{HttpDocumentMetadata, HttpInterface, RestHirDocument};
use crate::error::{ParseError, ParserResult};
use crate::hir;

impl RestHirDocument {
    /// Reads the projected HTTP document from parser properties.
    pub fn from_props(props: &hir::ParserProperties) -> ParserResult<Self> {
        let value = props
            .get("rest_hir")
            .cloned()
            .ok_or_else(|| ParseError::Message("missing rest_hir properties".to_string()))?;
        serde_json::from_value(value).map_err(|err| ParseError::Message(err.to_string()))
    }

    /// Finds an interface by its enclosing module path and name.
    pub fn find_interface(&self, module_path: &[String], name: &str) -> Option<&HttpInterface> {
        self.interfaces
            .iter()
            .find(|interface| interface.name == name && interface.module_path == module_path)
    }
}

impl HttpDocumentMetadata {
    /// Finds the version of an interface, including nested business modules.
    pub fn api_version(&self, module_path: &[String]) -> Option<&str> {
        self.version.as_deref().or_else(|| {
            module_path
                .first()
                .and_then(|module| self.version_values.get(module))
                .map(String::as_str)
        })
    }
}
