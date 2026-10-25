use crate::{
    gen::{v3, version_adapters::*},
    hooks::{Facts, Readers},
};
use serde_json::{json, Value};
use std::{collections::BTreeMap, sync::Mutex};

#[derive(Clone)]
struct Record {
    head: v3::shipping::Address,
    history: Vec<Facts>,
}

#[derive(Default)]
pub struct Store(Mutex<BTreeMap<(String, String), Record>>);

impl Store {
    pub fn seeded() -> Self {
        let mut rows = BTreeMap::new();
        for (tenant, id, reference) in [
            ("acme", "a1", Some("A-1")),
            ("acme", "a2", Some("A-2")),
            ("other", "a1", Some("B-1")),
            ("acme", "orphan", None),
        ] {
            rows.insert(
                (tenant.into(), id.into()),
                Record {
                    head: v3::shipping::Address {
                        base: v3::shipping::Postal {
                            base: v3::geo::Identity {
                                base: v3::geo::Anchor {
                                    base: v3::geo::Marker {},
                                    previous: Some(vec![v3::geo::Anchor {
                                        base: v3::geo::Marker {},
                                        previous: None,
                                    }]),
                                },
                                id: id.into(),
                                tenant: tenant.into(),
                                signature_required: Some(true),
                            },
                            street: "Shared street".into(),
                            country_code_v2: v3::geo::Country::US,
                        },
                        priority: 1,
                    },
                    history: vec![Facts {
                        reference: reference.map(str::to_string),
                        note: None,
                    }],
                },
            );
        }
        Self(Mutex::new(rows))
    }

    pub fn current(&self, tenant: &str, id: &str) -> Option<v3::shipping::Address> {
        self.0
            .lock()
            .unwrap()
            .get(&(tenant.into(), id.into()))
            .map(|row| row.head.clone())
    }

    pub fn readers(&self, tenant: &str) -> Readers {
        Readers(
            self.0
                .lock()
                .unwrap()
                .iter()
                .filter(|((owner, _), _)| owner == tenant)
                .map(|(key, record)| (key.clone(), record.history.last().unwrap().clone()))
                .collect(),
        )
    }

    /// Both front ends call this one Head implementation with a generated stored bag.
    pub fn update(
        &self,
        input: ShippingAddressesUpdateRequestV3,
        bag: ShippingAddressesUpdateStored,
    ) -> HeadOutcome<v3::shipping::Address, v3::shipping::AddressesUpdateError, Readers> {
        let mut rows = self.0.lock().unwrap();
        let row = rows.get_mut(&(input.tenant.clone(), input.id)).unwrap();
        let patch = input.patch;
        if let Some(street) = patch.base.street.or(bag.line) {
            row.head.base.street = street;
        }
        if let Some(country) = patch.base.country_code_v2 {
            row.head.base.country_code_v2 = country;
        }
        if let Some(priority) = patch.priority {
            row.head.priority = priority;
        }
        if let Some(signature) = patch.base.base.signature_required {
            row.head.base.base.signature_required = Some(signature);
        }
        let mut facts = row.history.last().unwrap().clone();
        if let Some(reference) = bag.delivery_reference {
            facts.reference = Some(reference);
        }
        if let Some(note) = bag.delivery_note {
            facts.note = Some(note);
        }
        // Old rows are never rewritten. Repeating the same view adds no history.
        if row.history.last() != Some(&facts) {
            row.history.push(facts);
        }
        let head = row.head.clone();
        drop(rows);
        HeadOutcome {
            outcome: Ok(head),
            stored: self.readers(&input.tenant),
        }
    }

    pub fn audit(&self) -> Value {
        let rows = self.0.lock().unwrap();
        json!(rows.iter().map(|((tenant, id), row)| json!({
            "tenant":tenant, "id":id, "head":row.head,
            "history":row.history.iter().map(|facts| json!({"reference":facts.reference,"note":facts.note})).collect::<Vec<_>>()
        })).collect::<Vec<_>>())
    }
}
