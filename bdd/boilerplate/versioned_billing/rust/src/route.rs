use crate::{
    gen::{v1, v2, version_adapters::*},
    ledger::Ledger,
    projection::Hooks,
};
use async_trait::async_trait;
use std::sync::Arc;

pub(crate) struct VersionRoute(pub(crate) Arc<Ledger>);

#[async_trait]
impl v1::Invoices for VersionRoute {
    async fn pay(
        &self,
        capture: v1::Capture,
        key: String,
        tenant: String,
    ) -> Result<v1::Invoice, v1::InvoicesPayError> {
        let input = invoices_pay_request_upcast_from_v_1(
            &InvoicesPayRequestV1 {
                capture,
                key,
                tenant,
            },
            &Hooks,
        )?;
        Hooks::project(downcast_invoices_pay_from_v_1_outcome(
            self.0.pay(input),
            &Hooks,
        ))
    }

    async fn get(&self, id: String, tenant: String) -> Result<v1::Invoice, v1::InvoicesGetError> {
        let input =
            invoices_get_request_upcast_from_v_1(&InvoicesGetRequestV1 { id, tenant }, &Hooks)?;
        Hooks::project(downcast_invoices_get_from_v_1_outcome(
            self.0.get(input),
            &Hooks,
        ))
    }

    async fn list(
        &self,
        limit: i32,
        tenant: String,
    ) -> Result<Vec<v1::Invoice>, v1::InvoicesListError> {
        let input = invoices_list_request_upcast_from_v_1(
            &InvoicesListRequestV1 { limit, tenant },
            &Hooks,
        )?;
        Hooks::project(downcast_invoices_list_from_v_1_outcome(
            self.0.list(input),
            &Hooks,
        ))
    }

    async fn audit(&self, tenant: String) -> Result<v1::Audit, v1::InvoicesAuditError> {
        let input =
            invoices_audit_request_upcast_from_v_1(&InvoicesAuditRequestV1 { tenant }, &Hooks)?;
        Hooks::project(downcast_invoices_audit_from_v_1_outcome(
            self.0.audit(input),
            &Hooks,
        ))
    }
}

#[async_trait]
impl v2::Invoices for VersionRoute {
    async fn pay(
        &self,
        capture: v2::Capture,
        key: String,
        tenant: String,
    ) -> Result<v2::Invoice, v2::InvoicesPayError> {
        self.0.pay(InvoicesPayRequestV2 {
            capture,
            key,
            tenant,
        })
    }

    async fn get(&self, id: String, tenant: String) -> Result<v2::Invoice, v2::InvoicesGetError> {
        self.0.get(InvoicesGetRequestV2 { id, tenant })
    }

    async fn list(
        &self,
        limit: i32,
        tenant: String,
    ) -> Result<Vec<v2::Invoice>, v2::InvoicesListError> {
        self.0.list(InvoicesListRequestV2 { limit, tenant })
    }

    async fn audit(&self, tenant: String) -> Result<v2::Audit, v2::InvoicesAuditError> {
        self.0.audit(InvoicesAuditRequestV2 { tenant })
    }
}
