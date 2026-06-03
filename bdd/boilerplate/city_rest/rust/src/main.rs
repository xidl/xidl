use async_trait::async_trait;
use std::env;

mod gen {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/city_rest.rs"));
}

struct SmartCityRestService;

#[async_trait]
impl gen::SmartCityRestApi for SmartCityRestService {
    async fn get_stop_eta<'a>(
        &'a self,
        stop_id: String,
        _line: String,
        _locale: String,
        _xidl_auth: xidl_rust_axum::auth::bearer::BearerAuth,
    ) -> Result<gen::SmartCityRestApiGetStopEtaResponse, xidl_rust_axum::Error> {
        Ok(gen::SmartCityRestApiGetStopEtaResponse {
            r#return: stop_id,
            eta_seconds: 240,
            destination: "Central Station".to_string(),
        })
    }

    async fn list_nearby_stops<'a>(
        &'a self,
        stop_id: String,
        _xidl_auth: xidl_rust_axum::auth::bearer::BearerAuth,
    ) -> Result<Vec<String>, xidl_rust_axum::Error> {
        Ok(vec![format!("{stop_id}-A"), format!("{stop_id}-B")])
    }

    async fn download_asset<'a>(
        &'a self,
        asset_path: String,
        _version: String,
        _xidl_auth: xidl_rust_axum::auth::bearer::BearerAuth,
    ) -> Result<gen::SmartCityRestApiDownloadAssetResponse, xidl_rust_axum::Error> {
        Ok(gen::SmartCityRestApiDownloadAssetResponse {
            r#return: format!("asset:{asset_path}").into_bytes(),
            content_type: "text/plain".to_string(),
            etag: "etag-demo".to_string(),
        })
    }

    async fn probe_lot<'a>(
        &'a self,
        _lot_id: String,
        _trace_id: String,
    ) -> Result<(), xidl_rust_axum::Error> {
        Ok(())
    }

    async fn reserve_lot<'a>(
        &'a self,
        lot_id: String,
        _plate_number: String,
        _minutes: u32,
        _xidl_auth: xidl_rust_axum::ApiKeyAuth,
    ) -> Result<gen::SmartCityRestApiReserveLotResponse, xidl_rust_axum::Error> {
        Ok(gen::SmartCityRestApiReserveLotResponse {
            r#return: format!("resv-{lot_id}"),
            reservation_state: "CONFIRMED".to_string(),
            expires_at: "2026-03-08T10:00:00Z".to_string(),
        })
    }

    async fn cancel_reservation<'a>(
        &'a self,
        _lot_id: String,
        _reservation_id: String,
        _xidl_auth: xidl_rust_axum::auth::bearer::BearerAuth,
    ) -> Result<(), xidl_rust_axum::Error> {
        Ok(())
    }

    async fn get_profile<'a>(
        &'a self,
        citizen_id: String,
        _xidl_auth: xidl_rust_axum::auth::bearer::BearerAuth,
    ) -> Result<gen::SmartCityRestApiGetProfileResponse, xidl_rust_axum::Error> {
        Ok(gen::SmartCityRestApiGetProfileResponse {
            r#return: citizen_id,
            display_name: "Taylor".to_string(),
            phone_number: "+1-555-0101".to_string(),
            language: "en-US".to_string(),
        })
    }

    async fn update_profile<'a>(
        &'a self,
        _citizen_id: String,
        _display_name: String,
        _phone_number: String,
        _xidl_auth: xidl_rust_axum::auth::bearer::BearerAuth,
    ) -> Result<gen::SmartCityRestApiUpdateProfileResponse, xidl_rust_axum::Error> {
        Ok(gen::SmartCityRestApiUpdateProfileResponse {
            audit_id: "audit-20260307-001".to_string(),
        })
    }

    async fn get_device_status<'a>(
        &'a self,
        device_id: String,
        trace_id: String,
        session_id: String,
        _locale: String,
        _xidl_auth: xidl_rust_axum::auth::bearer::BearerAuth,
    ) -> Result<gen::SmartCityRestApiGetDeviceStatusResponse, xidl_rust_axum::Error> {
        Ok(gen::SmartCityRestApiGetDeviceStatusResponse {
            r#return: format!("device:{device_id}"),
            trace_echo: trace_id,
            session_echo: session_id,
        })
    }

    async fn get_attribute_api_version(
        &self,
        _xidl_auth: xidl_rust_axum::auth::bearer::BearerAuth,
    ) -> Result<String, xidl_rust_axum::Error> {
        Ok("v2.0.0".to_string())
    }

    async fn get_attribute_maintenance_mode(
        &self,
        _xidl_auth: xidl_rust_axum::auth::bearer::BearerAuth,
    ) -> Result<bool, xidl_rust_axum::Error> {
        Ok(false)
    }

    async fn set_attribute_maintenance_mode(
        &self,
        _maintenance_mode: bool,
        _xidl_auth: xidl_rust_axum::auth::bearer::BearerAuth,
    ) -> Result<(), xidl_rust_axum::Error> {
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("127.0.0.1:{}", port);
    let svc = gen::SmartCityRestApiServer::new(SmartCityRestService);
    println!("Rust server starting on {}", addr);
    xidl_rust_axum::Server::builder()
        .with_service(svc)
        .serve(&addr)
        .await?;
    Ok(())
}
