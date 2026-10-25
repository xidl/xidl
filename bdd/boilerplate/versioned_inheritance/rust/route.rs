//! Ordinary generated routers decode their own versions before typed projection.
use crate::{
    gen::{v1, v2, v3, version_adapters::*},
    hooks::Hooks,
    store::Store,
};
use std::sync::Arc;

pub struct Facade(pub Arc<Store>);

impl Facade {
    fn from_v1(
        input: ShippingAddressesUpdateRequestV1,
        current: &v3::shipping::Address,
    ) -> VersionedResult<Upcast<ShippingAddressesUpdateRequestV3, ShippingAddressesUpdateStored>>
    {
        let middle = shipping_addresses_update_request_upcast_from_v_1(&input, current, &Hooks)?;
        let mut head =
            shipping_addresses_update_request_upcast_from_v_2(&middle.value, current, &Hooks)?;
        head.stored = middle.stored.merge(head.stored);
        Ok(head)
    }
    fn from_v2(
        input: ShippingAddressesUpdateRequestV2,
        current: &v3::shipping::Address,
    ) -> VersionedResult<Upcast<ShippingAddressesUpdateRequestV3, ShippingAddressesUpdateStored>>
    {
        shipping_addresses_update_request_upcast_from_v_2(&input, current, &Hooks)
    }
}

// The only differences are public version types and the selected pure functions.
macro_rules! version_route {
    ($v:ident, $request:ident, $up:ident, $read:ident, $update:ident) => {
        #[async_trait::async_trait]
        impl $v::shipping::Addresses for Facade {
            async fn read(
                &self,
                tenant: String,
                id: String,
            ) -> Result<$v::shipping::Address, $v::shipping::AddressesReadError> {
                let outcome = self.0.current(&tenant, &id).ok_or_else(|| {
                    v3::shipping::AddressesReadError::Missing(v3::shipping::Missing {
                        reason: "address missing".into(),
                    })
                });
                match $read(outcome, &self.0.readers(&tenant), &Hooks) {
                    ProjectionOutcome::Projected(value) => value,
                    ProjectionOutcome::Unsupported { error, .. } => Err(
                        $v::shipping::AddressesReadError::Projection($v::shipping::Projection {
                            type_path: error.type_path,
                            reason: error.reason,
                        }),
                    ),
                }
            }
            async fn update(
                &self,
                tenant: String,
                id: String,
                patch: $v::shipping::Change,
            ) -> Result<$v::shipping::Address, $v::shipping::AddressesUpdateError> {
                let current = self.0.current(&tenant, &id).ok_or_else(|| {
                    $v::shipping::AddressesUpdateError::Missing($v::shipping::Missing {
                        reason: "address missing".into(),
                    })
                })?;
                let input =
                    Self::$up($request { tenant, id, patch }, &current).map_err(|error| {
                        $v::shipping::AddressesUpdateError::Projection($v::shipping::Projection {
                            type_path: error.type_path,
                            reason: error.reason,
                        })
                    })?;
                let result = self.0.update(input.value, input.stored);
                match $update(result.outcome, &result.stored, &Hooks) {
                    ProjectionOutcome::Projected(value) => value,
                    ProjectionOutcome::Unsupported { error, .. } => Err(
                        $v::shipping::AddressesUpdateError::Projection($v::shipping::Projection {
                            type_path: error.type_path,
                            reason: error.reason,
                        }),
                    ),
                }
            }
        }
    };
}

version_route!(
    v1,
    ShippingAddressesUpdateRequestV1,
    from_v1,
    downcast_shipping_addresses_read_from_v_1_outcome,
    downcast_shipping_addresses_update_from_v_1_outcome
);
version_route!(
    v2,
    ShippingAddressesUpdateRequestV2,
    from_v2,
    downcast_shipping_addresses_read_from_v_2_outcome,
    downcast_shipping_addresses_update_from_v_2_outcome
);
#[async_trait::async_trait]
impl v3::shipping::Addresses for Facade {
    async fn read(
        &self,
        tenant: String,
        id: String,
    ) -> Result<v3::shipping::Address, v3::shipping::AddressesReadError> {
        self.0.current(&tenant, &id).ok_or_else(|| {
            v3::shipping::AddressesReadError::Missing(v3::shipping::Missing {
                reason: "address missing".into(),
            })
        })
    }
    async fn update(
        &self,
        tenant: String,
        id: String,
        patch: v3::shipping::Change,
    ) -> Result<v3::shipping::Address, v3::shipping::AddressesUpdateError> {
        if self.0.current(&tenant, &id).is_none() {
            return Err(v3::shipping::AddressesUpdateError::Missing(
                v3::shipping::Missing {
                    reason: "address missing".into(),
                },
            ));
        }
        self.0
            .update(
                ShippingAddressesUpdateRequestV3 { tenant, id, patch },
                ShippingAddressesUpdateStored::default(),
            )
            .outcome
    }
}
