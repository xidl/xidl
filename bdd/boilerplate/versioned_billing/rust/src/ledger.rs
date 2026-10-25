use crate::gen::{v2, version_adapters::*};
use std::{collections::BTreeMap, sync::Mutex};

struct Replay {
    capture: v2::Capture,
    outcome: v2::Invoice,
}

#[derive(Default)]
struct State {
    invoices: BTreeMap<(String, String), v2::Invoice>,
    replays: BTreeMap<(String, String), Replay>,
    audits: BTreeMap<String, v2::Audit>,
}

pub(crate) struct Ledger(Mutex<State>);

impl Ledger {
    pub(crate) fn new() -> Self {
        let mut state = State::default();
        for tenant in ["acme", "other"] {
            for id in ["inv_hurl", "inv_sdk", "inv_ts"] {
                state.invoices.insert(
                    (tenant.into(), id.into()),
                    v2::Invoice {
                        id: id.into(),
                        total: 1000,
                        amount_paid: 0,
                        payments: Vec::new(),
                    },
                );
            }
        }
        Self(Mutex::new(state))
    }

    fn authorize(tenant: &str) -> Result<(), v2::Denied> {
        if matches!(tenant, "acme" | "other") {
            Ok(())
        } else {
            Err(v2::Denied {
                reason: "tenant is not authorized".into(),
            })
        }
    }

    pub(crate) fn pay(
        &self,
        input: InvoicesPayRequestV2,
    ) -> Result<v2::Invoice, v2::InvoicesPayError> {
        Self::authorize(&input.tenant).map_err(v2::InvoicesPayError::Denied)?;
        let mut state = self.0.lock().expect("ledger lock");
        state
            .audits
            .entry(input.tenant.clone())
            .or_default()
            .attempts += 1;
        if input.key.is_empty() {
            return Err(v2::InvoicesPayError::Rejected(v2::Rejected {
                reason: "idempotency key must not be empty".into(),
                type_path: "key".into(),
            }));
        }
        let replay_key = (input.tenant.clone(), input.key);
        if let Some(replay) = state.replays.get(&replay_key) {
            return if replay.capture == input.capture {
                Ok(replay.outcome.clone())
            } else {
                Err(v2::InvoicesPayError::Conflict(v2::Conflict {
                    reason: "idempotency key reused with a different payment".into(),
                }))
            };
        }
        let invoice_key = (input.tenant.clone(), input.capture.invoice_id.clone());
        let invoice = state.invoices.get_mut(&invoice_key).ok_or_else(|| {
            v2::InvoicesPayError::NotFound(v2::NotFound {
                id: input.capture.invoice_id.clone(),
            })
        })?;
        let payment = &input.capture.payment;
        if payment.amount <= 0 || payment.amount > invoice.total - invoice.amount_paid {
            return Err(v2::InvoicesPayError::Rejected(v2::Rejected {
                reason: "payment must be positive and no greater than the balance".into(),
                type_path: "Capture.payment.amount".into(),
            }));
        }
        if payment.payment_intent.is_empty()
            || invoice
                .payments
                .iter()
                .any(|item| item.payment_intent == payment.payment_intent)
        {
            return Err(v2::InvoicesPayError::Conflict(v2::Conflict {
                reason: "payment intent is missing or already captured".into(),
            }));
        }
        invoice.amount_paid += payment.amount;
        invoice.payments.push(payment.clone());
        let outcome = invoice.clone();
        state.replays.insert(
            replay_key,
            Replay {
                capture: input.capture,
                outcome: outcome.clone(),
            },
        );
        state.audits.entry(input.tenant).or_default().commits += 1;
        Ok(outcome)
    }

    pub(crate) fn get(
        &self,
        input: InvoicesGetRequestV2,
    ) -> Result<v2::Invoice, v2::InvoicesGetError> {
        Self::authorize(&input.tenant).map_err(v2::InvoicesGetError::Denied)?;
        self.0
            .lock()
            .expect("ledger lock")
            .invoices
            .get(&(input.tenant, input.id.clone()))
            .cloned()
            .ok_or(v2::InvoicesGetError::NotFound(v2::NotFound {
                id: input.id,
            }))
    }

    pub(crate) fn list(
        &self,
        input: InvoicesListRequestV2,
    ) -> Result<Vec<v2::Invoice>, v2::InvoicesListError> {
        Self::authorize(&input.tenant).map_err(v2::InvoicesListError::Denied)?;
        if !(0..=100).contains(&input.limit) {
            return Err(v2::InvoicesListError::Rejected(v2::Rejected {
                reason: "limit must be between zero and 100".into(),
                type_path: "limit".into(),
            }));
        }
        Ok(self
            .0
            .lock()
            .expect("ledger lock")
            .invoices
            .iter()
            .filter(|((tenant, _), _)| tenant == &input.tenant)
            .take(input.limit as usize)
            .map(|(_, invoice)| invoice.clone())
            .collect())
    }

    pub(crate) fn audit(
        &self,
        input: InvoicesAuditRequestV2,
    ) -> Result<v2::Audit, v2::InvoicesAuditError> {
        Self::authorize(&input.tenant).map_err(v2::InvoicesAuditError::Denied)?;
        Ok(self
            .0
            .lock()
            .expect("ledger lock")
            .audits
            .get(&input.tenant)
            .cloned()
            .unwrap_or_default())
    }
}
