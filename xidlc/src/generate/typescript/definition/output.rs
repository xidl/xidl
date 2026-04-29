#[derive(Default)]
pub(crate) struct TsRenderOutput {
    pub(crate) types: Vec<String>,
    pub(crate) zod: Vec<String>,
    pub(crate) client: Vec<String>,
}

impl TsRenderOutput {
    pub(crate) fn extend(&mut self, other: TsRenderOutput) {
        self.types.extend(other.types);
        self.zod.extend(other.zod);
        self.client.extend(other.client);
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.types.is_empty() && self.zod.is_empty() && self.client.is_empty()
    }
}
