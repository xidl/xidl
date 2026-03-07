include!(concat!(env!("OUT_DIR"), "/city_jsonrpc.rs"));

use std::sync::Mutex;

pub struct SmartCityRpcService {
    firmware_channel: Mutex<String>,
}

impl Default for SmartCityRpcService {
    fn default() -> Self {
        Self {
            firmware_channel: Mutex::new("stable".to_string()),
        }
    }
}

#[async_trait::async_trait]
impl SmartCityRpcApi for SmartCityRpcService {
    async fn quote_trip(
        &self,
        rider_id: String,
        zone_id: String,
    ) -> Result<SmartCityRpcApiquoteTripResult, xidl_jsonrpc::Error> {
        Ok(SmartCityRpcApiquoteTripResult {
            r#return: format!("quote-{rider_id}-{zone_id}"),
            amount_cents: 1880,
            currency: "CNY".to_string(),
        })
    }

    async fn create_invoice(
        &self,
        rider_id: String,
        amount_cents: i32,
        currency: String,
    ) -> Result<SmartCityRpcApicreateInvoiceResult, xidl_jsonrpc::Error> {
        let invoice_id = format!("inv-{rider_id}-{amount_cents}");
        Ok(SmartCityRpcApicreateInvoiceResult {
            r#return: "created".to_string(),
            invoice_id: invoice_id.clone(),
            payment_url: format!("https://pay.example.com/{invoice_id}?ccy={currency}"),
        })
    }

    async fn mark_paid(&self, _invoice_id: String) -> Result<(), xidl_jsonrpc::Error> {
        Ok(())
    }

    async fn heartbeat(&self) -> Result<(), xidl_jsonrpc::Error> {
        Ok(())
    }

    async fn rotate_session(
        &self,
        session_token: String,
    ) -> Result<SmartCityRpcApirotateSessionResult, xidl_jsonrpc::Error> {
        Ok(SmartCityRpcApirotateSessionResult {
            session_token: format!("{session_token}-next"),
            expires_at_epoch_sec: 1_710_000_000,
        })
    }

    async fn dispatch(
        &self,
        vehicle_id: String,
        pickup_stop: String,
    ) -> Result<SmartCityRpcApidispatchResult, xidl_jsonrpc::Error> {
        Ok(SmartCityRpcApidispatchResult {
            r#return: 12,
            job_id: format!("job-{vehicle_id}-{pickup_stop}"),
        })
    }

    async fn report_trip(
        &self,
        _order_id: String,
        _rider_id: String,
        _status: String,
    ) -> Result<(), xidl_jsonrpc::Error> {
        Ok(())
    }

    async fn summarize(
        &self,
        _day: String,
    ) -> Result<SmartCityRpcApisummarizeResult, xidl_jsonrpc::Error> {
        Ok(SmartCityRpcApisummarizeResult {
            trip_count: 42,
            gross_revenue_cents: 123_456,
        })
    }

    async fn get_attribute_region(&self) -> Result<String, xidl_jsonrpc::Error> {
        Ok("cn-east".to_string())
    }

    async fn get_attribute_firmware_channel(&self) -> Result<String, xidl_jsonrpc::Error> {
        let channel = self
            .firmware_channel
            .lock()
            .map_err(|err| xidl_jsonrpc::Error::Rpc {
                code: xidl_jsonrpc::ErrorCode::InternalError,
                message: err.to_string(),
                data: None,
            })?;
        Ok(channel.clone())
    }

    async fn set_attribute_firmware_channel(
        &self,
        firmware_channel: String,
    ) -> Result<(), xidl_jsonrpc::Error> {
        let mut channel = self
            .firmware_channel
            .lock()
            .map_err(|err| xidl_jsonrpc::Error::Rpc {
                code: xidl_jsonrpc::ErrorCode::InternalError,
                message: err.to_string(),
                data: None,
            })?;
        *channel = firmware_channel;
        Ok(())
    }
}
