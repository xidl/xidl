use crate::gen::{v1, v2, version_adapters::*};

pub(crate) struct Hooks;

impl VersionAdapters for Hooks {
    fn product_downcast_to_v_1(
        &self,
        source: &v2::Product,
    ) -> VersionedResult<ProductDowncastToV1Decision> {
        Ok(ProductDowncastToV1Decision {
            images: source
                .assets
                .iter()
                .filter(|asset| asset.kind == "image")
                .map(|asset| v1::Image {
                    url: asset.url.clone(),
                })
                .collect(),
        })
    }
}

impl From<UnsupportedProjection> for v1::CatalogSearchError {
    fn from(error: UnsupportedProjection) -> Self {
        Self::Framework(xidl_rust_axum::Error::new(500, error.reason))
    }
}
