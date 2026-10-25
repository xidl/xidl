use crate::gen::{v1, v2, v3, version_adapters::*};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Facts {
    pub reference: Option<String>,
    pub note: Option<String>,
}

/// A tenant-limited snapshot: readers use the stable identity of the derived object.
#[derive(Clone, Default)]
pub struct Readers(pub BTreeMap<(String, String), Facts>);

impl Readers {
    fn facts(&self, source: &v3::shipping::Address) -> Option<&Facts> {
        let identity = &source.base.base;
        self.0.get(&(identity.tenant.clone(), identity.id.clone()))
    }
}

impl StoredValues for Readers {
    fn shipping_address_country_code(&self, source: &v2::shipping::Address) -> Option<String> {
        // This information is still present. No historical lookup is needed.
        Some(
            match source.base.country_code_v2 {
                v2::geo::Country::US => "US",
                v2::geo::Country::CA => "CA",
            }
            .into(),
        )
    }
    fn shipping_address_delivery_reference(
        &self,
        source: &v3::shipping::Address,
    ) -> Option<String> {
        self.facts(source).and_then(|facts| facts.reference.clone())
    }
    fn shipping_address_delivery_note(&self, source: &v3::shipping::Address) -> Option<String> {
        self.facts(source).and_then(|facts| facts.note.clone())
    }
}

pub struct Hooks;

impl Hooks {
    pub fn refusal(direction: Direction, path: &str) -> UnsupportedProjection {
        UnsupportedProjection {
            version: "2026-04-01".into(),
            direction,
            type_path: path.into(),
            reason: "address has no faithful representation".into(),
        }
    }
}

impl VersionAdapters for Hooks {
    fn shipping_address_downcast_to_v_1(
        &self,
        source: &v2::shipping::Address,
        stored: &dyn StoredValues,
    ) -> VersionedResult<ShippingAddressDowncastToV1Decision> {
        Ok(ShippingAddressDowncastToV1Decision {
            country_code: stored
                .shipping_address_country_code(source)
                .ok_or_else(|| {
                    Self::refusal(Direction::Downcast, "shipping::Address.countryCode")
                })?,
        })
    }
    fn shipping_address_downcast_to_v_2(
        &self,
        source: &v3::shipping::Address,
        stored: &dyn StoredValues,
    ) -> VersionedResult<ShippingAddressDowncastToV2Decision> {
        Ok(ShippingAddressDowncastToV2Decision {
            delivery_reference: stored
                .shipping_address_delivery_reference(source)
                .ok_or_else(|| {
                    Self::refusal(Direction::Downcast, "shipping::Address.deliveryReference")
                })?,
            delivery_note: stored.shipping_address_delivery_note(source),
        })
    }
    fn shipping_change_upcast_from_v_1(
        &self,
        source: &v1::shipping::Change,
        _: &v3::shipping::Address,
    ) -> VersionedResult<ShippingChangeUpcastFromV1Decision> {
        let country_code_v2 = source
            .base
            .country_code
            .as_deref()
            .map(|code| match code {
                "US" => Ok(v2::geo::Country::US),
                "CA" => Ok(v2::geo::Country::CA),
                _ => Err(Self::refusal(
                    Direction::Upcast,
                    "shipping::Change.countryCode",
                )),
            })
            .transpose()?;
        Ok(ShippingChangeUpcastFromV1Decision {
            country_code_v2,
            country_code: IGNORED,
            stored: ShippingAddressesUpdateStored::default(),
        })
    }
    fn shipping_change_upcast_from_v_2(
        &self,
        source: &v2::shipping::Change,
        current: &v3::shipping::Address,
    ) -> VersionedResult<ShippingChangeUpcastFromV2Decision> {
        Ok(ShippingChangeUpcastFromV2Decision {
            // A narrow PATCH cannot overwrite a choice its caller cannot see.
            signature_required: current.base.base.signature_required.unwrap_or(false),
            delivery_reference: IGNORED,
            delivery_note: IGNORED,
            stored: ShippingAddressesUpdateStored {
                delivery_reference: source.base.base.delivery_reference.clone(),
                delivery_note: source.base.base.delivery_note.clone(),
                ..Default::default()
            },
        })
    }
}
