/// Declaration scope used while rendering OpenAPI schemas.
#[derive(Clone, Copy)]
pub(crate) struct SchemaScope<'a> {
    pub(crate) module_path: &'a [String],
}

impl<'a> SchemaScope<'a> {
    pub(crate) fn new(module_path: &'a [String]) -> Self {
        Self { module_path }
    }
}
