use crate::{
    catalog::Catalog,
    gen::{v1, v2, version_adapters::*},
    projection::Hooks,
};
use async_trait::async_trait;
use std::sync::Arc;

pub(crate) struct VersionRoute(pub(crate) Arc<Catalog>);

#[async_trait]
impl v1::Catalog for VersionRoute {
    async fn search(
        &self,
        media: Vec<String>,
        tenant: String,
    ) -> Result<v1::Page, v1::CatalogSearchError> {
        let input = catalog_search_request_upcast_from_v_1(
            &CatalogSearchRequestV1 { media, tenant },
            &Hooks,
        )?;
        match downcast_catalog_search_from_v_1_outcome(self.0.search(input), &Hooks) {
            ProjectionOutcome::Projected(result) => result,
            ProjectionOutcome::Unsupported { error, .. } => Err(error.into()),
        }
    }
}

#[async_trait]
impl v2::Catalog for VersionRoute {
    async fn search(
        &self,
        media: Vec<String>,
        tenant: String,
    ) -> Result<v2::Page, v2::CatalogSearchError> {
        self.0.search(CatalogSearchRequestV2 { media, tenant })
    }
}
