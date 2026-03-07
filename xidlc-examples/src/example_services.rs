use crate::city_http::{
    SmartCityHttpApi, SmartCityHttpApiCancelReservationRequest,
    SmartCityHttpApiDownloadAssetRequest, SmartCityHttpApiDownloadAssetResponse,
    SmartCityHttpApiGetProfileRequest, SmartCityHttpApiGetProfileResponse,
    SmartCityHttpApiGetStopEtaRequest, SmartCityHttpApiGetStopEtaResponse,
    SmartCityHttpApiListNearbyStopsRequest, SmartCityHttpApiProbeLotRequest,
    SmartCityHttpApiReserveLotRequest, SmartCityHttpApiReserveLotResponse,
    SmartCityHttpApiSetMaintenanceModeRequest, SmartCityHttpApiUpdateProfileRequest,
};
use serde_json::json;

pub struct SmartCityHttpService;

#[async_trait::async_trait]
impl SmartCityHttpApi for SmartCityHttpService {
    async fn get_stop_eta(
        &self,
        req: xidl_rust_axum::Request<SmartCityHttpApiGetStopEtaRequest>,
    ) -> Result<SmartCityHttpApiGetStopEtaResponse, xidl_rust_axum::Error> {
        let data = req.data;
        Ok(SmartCityHttpApiGetStopEtaResponse {
            r#return: data.stop_id,
            eta_seconds: 240,
            destination: "Central Station".to_string(),
        })
    }

    async fn list_nearby_stops(
        &self,
        req: xidl_rust_axum::Request<SmartCityHttpApiListNearbyStopsRequest>,
    ) -> Result<Vec<String>, xidl_rust_axum::Error> {
        Ok(vec![
            format!("{}-A", req.data.stop_id),
            format!("{}-B", req.data.stop_id),
        ])
    }

    async fn download_asset(
        &self,
        req: xidl_rust_axum::Request<SmartCityHttpApiDownloadAssetRequest>,
    ) -> Result<SmartCityHttpApiDownloadAssetResponse, xidl_rust_axum::Error> {
        Ok(SmartCityHttpApiDownloadAssetResponse {
            r#return: format!("asset:{}", req.data.asset_path).into_bytes(),
            content_type: "text/plain".to_string(),
            etag: "etag-demo".to_string(),
        })
    }

    async fn probe_lot(
        &self,
        _req: xidl_rust_axum::Request<SmartCityHttpApiProbeLotRequest>,
    ) -> Result<(), xidl_rust_axum::Error> {
        Ok(())
    }

    async fn reserve_lot(
        &self,
        req: xidl_rust_axum::Request<SmartCityHttpApiReserveLotRequest>,
    ) -> Result<SmartCityHttpApiReserveLotResponse, xidl_rust_axum::Error> {
        Ok(SmartCityHttpApiReserveLotResponse {
            r#return: format!("resv-{}", req.data.lot_id),
            reservation_state: "CONFIRMED".to_string(),
            expires_at: "2026-03-08T10:00:00Z".to_string(),
        })
    }

    async fn cancel_reservation(
        &self,
        _req: xidl_rust_axum::Request<SmartCityHttpApiCancelReservationRequest>,
    ) -> Result<(), xidl_rust_axum::Error> {
        Ok(())
    }

    async fn get_profile(
        &self,
        req: xidl_rust_axum::Request<SmartCityHttpApiGetProfileRequest>,
    ) -> Result<SmartCityHttpApiGetProfileResponse, xidl_rust_axum::Error> {
        Ok(SmartCityHttpApiGetProfileResponse {
            r#return: req.data.citizen_id,
            display_name: "Taylor".to_string(),
            phone_number: "+1-555-0101".to_string(),
            language: "en-US".to_string(),
        })
    }

    async fn update_profile(
        &self,
        _req: xidl_rust_axum::Request<SmartCityHttpApiUpdateProfileRequest>,
    ) -> Result<String, xidl_rust_axum::Error> {
        Ok("audit-20260307-001".to_string())
    }

    async fn api_version(
        &self,
        _req: xidl_rust_axum::Request<()>,
    ) -> Result<String, xidl_rust_axum::Error> {
        Ok("v2.0.0".to_string())
    }

    async fn maintenance_mode(
        &self,
        _req: xidl_rust_axum::Request<()>,
    ) -> Result<bool, xidl_rust_axum::Error> {
        Ok(false)
    }

    async fn set_maintenance_mode(
        &self,
        _req: xidl_rust_axum::Request<SmartCityHttpApiSetMaintenanceModeRequest>,
    ) -> Result<(), xidl_rust_axum::Error> {
        Ok(())
    }
}

pub struct SmartCityRpcService;

#[async_trait::async_trait]
impl xidl_jsonrpc::Handler for SmartCityRpcService {
    async fn handle(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, xidl_jsonrpc::Error> {
        match method {
            "SmartCityRpcApi.mark_paid" => Ok(json!({})),
            "SmartCityRpcApi.heartbeat" => Ok(json!({})),
            "SmartCityRpcApi.report_trip" => Ok(json!({})),
            "SmartCityRpcApi.get_attribute_region" => Ok(json!({ "return": "cn-east" })),
            "SmartCityRpcApi.get_attribute_firmware_channel" => Ok(json!({ "return": "stable" })),
            "SmartCityRpcApi.set_attribute_firmware_channel" => {
                let _channel = params
                    .get("firmware_channel")
                    .and_then(|value| value.as_str())
                    .unwrap_or("stable");
                Ok(json!({}))
            }
            _ => Err(xidl_jsonrpc::Error::method_not_found(method)),
        }
    }
}
