use crate::gen::{
    v2,
    version_adapters::{CatalogRememberRequestV2, CatalogSearchRequestV2},
};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) struct Catalog {
    tenants: BTreeMap<String, Vec<v2::Product>>,
}

impl Catalog {
    pub(crate) fn new() -> Self {
        Self {
            tenants: BTreeMap::from([
                (
                    "shop-a".into(),
                    vec![
                        Self::product("image", &["image"]),
                        Self::product("clip", &["image", "video"]),
                        Self::product("model", &["image", "video", "model_3d"]),
                        Self::product("video-only", &["video"]),
                    ],
                ),
                ("shop-b".into(), vec![Self::product("private", &["video"])]),
            ]),
        }
    }

    fn product(id: &str, kinds: &[&str]) -> v2::Product {
        v2::Product {
            id: id.into(),
            assets: kinds
                .iter()
                .map(|kind| v2::Media {
                    kind: (*kind).into(),
                    url: format!("https://assets.example/{id}/{kind}"),
                })
                .collect(),
        }
    }

    pub(crate) fn search(
        &self,
        input: CatalogSearchRequestV2,
    ) -> Result<v2::CatalogSearchResponse, v2::CatalogSearchError> {
        let products = self
            .tenant(&input.tenant)
            .map_err(v2::CatalogSearchError::Denied)?;
        let bookmark = Self::preference(input.remember).map_err(v2::CatalogSearchError::Denied)?;
        let mut filters = BTreeSet::new();
        let mut unknown = BTreeSet::new();
        let mut messages = Vec::new();
        for kind in &input.media {
            match kind.as_str() {
                "image" | "video" | "model_3d" => {
                    filters.insert(kind.as_str());
                }
                _ if unknown.insert(kind.as_str()) => messages.push(v2::Message {
                    code: "not_found".into(),
                    value: kind.clone(),
                }),
                _ => {}
            }
        }
        // Matching uses complete Head facts, including media old clients cannot see.
        let products = products
            .iter()
            .filter(|product| {
                filters
                    .iter()
                    .all(|kind| product.assets.iter().any(|asset| asset.kind == *kind))
            })
            .cloned()
            .collect();
        Ok(v2::CatalogSearchResponse {
            r#return: v2::Page { products, messages },
            bookmark,
        })
    }

    pub(crate) fn remember(
        &self,
        input: CatalogRememberRequestV2,
    ) -> Result<v2::CatalogRememberResponse, v2::CatalogRememberError> {
        self.tenant(&input.tenant)
            .map_err(v2::CatalogRememberError::Denied)?;
        Ok(v2::CatalogRememberResponse {
            bookmark: Self::preference(input.media).map_err(v2::CatalogRememberError::Denied)?,
        })
    }

    fn tenant(&self, tenant: &str) -> Result<&Vec<v2::Product>, v2::Denied> {
        self.tenants.get(tenant).ok_or_else(|| v2::Denied {
            reason: "unknown catalog tenant".into(),
        })
    }

    fn preference(media: Option<String>) -> Result<Option<String>, v2::Denied> {
        match media.as_deref() {
            None | Some("" | "image" | "video" | "model_3d") => Ok(media),
            Some(_) => Err(v2::Denied {
                reason: "unknown media preference".into(),
            }),
        }
    }
}
