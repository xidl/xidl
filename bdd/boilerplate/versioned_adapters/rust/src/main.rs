use async_trait::async_trait;
use std::env;

mod gen {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/{{MODULE_NAME}}.rs"));
}

/// The one business implementation: the head version, and nothing else.
#[derive(Default)]
struct HeadItems;

#[async_trait]
impl gen::v2::Items for HeadItems {
    async fn get(&self, id: String) -> ::std::result::Result<gen::v2::Item, gen::v2::ItemsGetError> {
        if id == "locked" {
            return Err(gen::v2::ItemsGetError::Locked(gen::v2::Locked {
                owner: "ops".to_string(),
            }));
        }
        Ok(gen::v2::Item {
            name: id.clone(),
            owner: "ops".to_string(),
            note: Some("head note".to_string()),
            ref_v2: format!("ref-{id}"),
        })
    }

    async fn create(
        &self,
        item: gen::v2::Item,
        _actor: String,
    ) -> ::xidl_rust_axum::Result<gen::v2::State> {
        if item.ref_v2 == "special" {
            Ok(gen::v2::State::archived)
        } else {
            Ok(gen::v2::State::draft)
        }
    }
}

/// The hook decisions the compiler could not make.
#[derive(Default)]
struct Adapters;

fn refused(reason: &str) -> gen::version_adapters::UnsupportedProjection {
    gen::version_adapters::UnsupportedProjection {
        version: "1.0".to_string(),
        direction: gen::version_adapters::Direction::Downcast,
        type_path: "State".to_string(),
        reason: reason.to_string(),
    }
}

fn refused_error(error: gen::version_adapters::UnsupportedProjection) -> xidl_rust_axum::Error {
    xidl_rust_axum::Error::new(500, format!("unprojectable: {}", error.reason))
}

impl gen::version_adapters::VersionAdapters for Adapters {
    fn item_upcast_from_v1(
        &self,
        source: &gen::v1::Item,
    ) -> gen::version_adapters::VersionedResult<gen::version_adapters::ItemUpcastDecisionFromV1> {
        Ok(gen::version_adapters::ItemUpcastDecisionFromV1 {
            color: gen::version_adapters::IGNORED,
            ref_v1: gen::version_adapters::IGNORED,
            owner: "ops".to_string(),
            ref_v2: source.ref_v1.clone(),
        })
    }

    fn item_downcast_to_v1(
        &self,
        source: &gen::v2::Item,
    ) -> gen::version_adapters::VersionedResult<gen::version_adapters::ItemDowncastDecisionToV1> {
        Ok(gen::version_adapters::ItemDowncastDecisionToV1 {
            ref_v1: source.ref_v2.clone(),
        })
    }

    fn state_downcast_to_v1(
        &self,
        value: &gen::v2::State,
    ) -> gen::version_adapters::VersionedResult<gen::v1::State> {
        match value {
            gen::v2::State::archived => Err(refused(
                "archived has no representation a v1 client can read",
            )),
            gen::v2::State::draft => Ok(gen::v1::State::draft),
        }
    }

    fn items_create_upcast_from_v1(
        &self,
        _source: &gen::v1::Item,
    ) -> gen::version_adapters::VersionedResult<gen::version_adapters::ItemsCreateUpcastDecisionFromV1> {
        Ok(gen::version_adapters::ItemsCreateUpcastDecisionFromV1 {
            actor: "v1-gateway".to_string(),
        })
    }

    fn items_get_locked_downcast_to_v1(
        &self,
        head: &gen::v2::Locked,
    ) -> gen::version_adapters::VersionedResult<gen::v1::ItemsGetError> {
        // A v1 client only knows NotFound for a locked item.
        Ok(gen::v1::ItemsGetError::NotFound(gen::v1::NotFound {
            id: head.owner.clone(),
        }))
    }
}

/// Serves v1 by composing the v1->v2 step around the head implementation.
#[derive(Default)]
struct V1Items {
    head: HeadItems,
    adapters: Adapters,
}

#[async_trait]
impl gen::v1::Items for V1Items {
    async fn get(&self, id: String) -> ::std::result::Result<gen::v1::Item, gen::v1::ItemsGetError> {
        let request = gen::version_adapters::upcast_items_get_from_v1(id, &self.adapters)
            .map_err(|error| gen::v1::ItemsGetError::Framework(refused_error(error)))?;
        let item = match gen::v2::Items::get(&self.head, request.id).await {
            Ok(item) => item,
            Err(error) => {
                let projected = gen::version_adapters::downcast_items_get_error_to_v1(
                    error,
                    &self.adapters,
                )
                .map_err(|error| gen::v1::ItemsGetError::Framework(refused_error(error)))?;
                return Err(projected);
            }
        };
        gen::version_adapters::downcast_items_get_result_to_v1(item, &self.adapters)
            .map_err(|error| gen::v1::ItemsGetError::Framework(refused_error(error)))
    }

    async fn create(&self, item: gen::v1::Item) -> ::xidl_rust_axum::Result<gen::v1::State> {
        let request = gen::version_adapters::upcast_items_create_from_v1(item, &self.adapters)
            .map_err(|error| refused_error(error))?;
        let state = gen::v2::Items::create(&self.head, request.item, request.actor).await?;
        gen::version_adapters::downcast_items_create_result_to_v1(state, &self.adapters)
            .map_err(|error| refused_error(error))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port_v1 = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let port_v2 = env::var("PORT_V2").unwrap_or_else(|_| "8081".to_string());
    let addr_v1 = format!("127.0.0.1:{port_v1}");
    let addr_v2 = format!("127.0.0.1:{port_v2}");
    println!("Adapter rust server starting on {addr_v1} (v1) and {addr_v2} (v2)");

    let v1 = xidl_rust_axum::Server::builder()
        .with_service(gen::v1::ItemsServer::new(V1Items::default()))
        .serve(addr_v1);
    let v2 = xidl_rust_axum::Server::builder()
        .with_service(gen::v2::ItemsServer::new(HeadItems))
        .serve(addr_v2);
    tokio::try_join!(v1, v2)?;
    Ok(())
}
