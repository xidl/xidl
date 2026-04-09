use crate::Path;
use serde::{Deserialize, Serialize};

use super::{HttpMethod, Operation, PathItem, PathItemBuilder, PathsMap};
use crate::openapi::{builder, extensions::Extensions, set_value};

builder! {
    PathsBuilder;

    #[non_exhaustive]
    #[derive(Serialize, Deserialize, Default, Clone, PartialEq)]
    pub struct Paths {
        #[serde(flatten)]
        pub paths: PathsMap<String, PathItem>,
        #[serde(skip_serializing_if = "Option::is_none", flatten)]
        pub extensions: Option<Extensions>,
    }
}

impl Paths {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn get_path_item<P: AsRef<str>>(&self, path: P) -> Option<&PathItem> {
        self.paths.get(path.as_ref())
    }

    pub fn get_path_operation<P: AsRef<str>>(
        &self,
        path: P,
        http_method: HttpMethod,
    ) -> Option<&Operation> {
        self.paths
            .get(path.as_ref())
            .and_then(|path| path.operation(http_method))
    }

    pub fn add_path_operation<P: AsRef<str>, O: Into<Operation>>(
        &mut self,
        path: P,
        http_methods: Vec<HttpMethod>,
        operation: O,
    ) {
        let path = path.as_ref();
        let operation = operation.into();
        if let Some(existing_item) = self.paths.get_mut(path) {
            existing_item.set_operations(http_methods, operation);
        } else {
            self.paths.insert(
                String::from(path),
                PathItem::from_http_methods(http_methods, operation),
            );
        }
    }

    pub fn merge(&mut self, other_paths: Paths) {
        for (path, that) in other_paths.paths {
            if let Some(this) = self.paths.get_mut(&path) {
                this.merge_operations(that);
            } else {
                self.paths.insert(path, that);
            }
        }

        if let Some(other_paths_extensions) = other_paths.extensions {
            let paths_extensions = self.extensions.get_or_insert(Extensions::default());
            paths_extensions.merge(other_paths_extensions);
        }
    }
}

impl PathsBuilder {
    pub fn path<I: Into<String>>(mut self, path: I, item: PathItem) -> Self {
        let path_string = path.into();
        if let Some(existing_item) = self.paths.get_mut(&path_string) {
            existing_item.merge_operations(item);
        } else {
            self.paths.insert(path_string, item);
        }

        self
    }

    pub fn extensions(mut self, extensions: Option<Extensions>) -> Self {
        set_value!(self extensions extensions)
    }

    pub fn path_from<P: Path>(self) -> Self {
        let methods = P::methods();
        let operation = P::operation();
        let path_item = if methods.len() == 1 {
            PathItem::new(
                methods
                    .into_iter()
                    .next()
                    .expect("must have one operation method"),
                operation,
            )
        } else {
            methods
                .into_iter()
                .fold(PathItemBuilder::new(), |path_item, method| {
                    path_item.operation(method, operation.clone())
                })
                .build()
        };

        self.path(P::path(), path_item)
    }
}
