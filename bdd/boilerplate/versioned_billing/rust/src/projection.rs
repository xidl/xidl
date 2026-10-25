use crate::gen::{v1, v2, version_adapters::*};

pub(crate) struct Hooks;

impl VersionAdapters for Hooks {
    fn invoice_downcast_to_v_1(
        &self,
        source: &v2::Invoice,
    ) -> VersionedResult<InvoiceDowncastToV1Decision> {
        let payment_intent = match source.payments.as_slice() {
            [] => None,
            [payment] => Some(payment.payment_intent.clone()),
            _ => {
                return Err(UnsupportedProjection {
                    version: Version::V1.value().into(),
                    direction: Direction::Downcast,
                    type_path: "Invoice.payment_intent".into(),
                    reason: "multiple payments have no singular payment intent".into(),
                })
            }
        };
        Ok(InvoiceDowncastToV1Decision { payment_intent })
    }
}

impl Hooks {
    pub(crate) fn rejected(error: UnsupportedProjection) -> v1::Rejected {
        v1::Rejected {
            reason: error.reason,
            type_path: error.type_path,
        }
    }

    pub(crate) fn project<T, E, H>(outcome: ProjectionOutcome<Result<T, E>, H>) -> Result<T, E>
    where
        E: From<UnsupportedProjection>,
    {
        match outcome {
            ProjectionOutcome::Projected(result) => result,
            ProjectionOutcome::Unsupported { error, .. } => Err(error.into()),
        }
    }
}

macro_rules! rejected_projection {
    ($($error:ident),+ $(,)?) => {$(
        impl From<UnsupportedProjection> for v1::$error {
            fn from(error: UnsupportedProjection) -> Self { Self::Rejected(Hooks::rejected(error)) }
        }
    )+};
}
rejected_projection!(InvoicesPayError, InvoicesGetError, InvoicesListError);

impl From<UnsupportedProjection> for v1::InvoicesAuditError {
    fn from(error: UnsupportedProjection) -> Self {
        Self::Framework(xidl_rust_axum::Error::new(500, error.reason))
    }
}
