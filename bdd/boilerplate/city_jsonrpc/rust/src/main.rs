use async_trait::async_trait;
mod gen { include!("../{{MODULE_NAME}}.rs"); }
struct MySmartCity;
#[async_trait]
impl gen::SmartCityRpcApi for MySmartCity {
    async fn quote_trip<'a>(&'a self, _rider_id: String, _zone_id: String) -> Result<gen::SmartCityRpcApiquoteTripResult, xidl_jsonrpc::Error> { Ok(gen::SmartCityRpcApiquoteTripResult { amount_cents: 100, currency: "USD".into(), r#return: "quote-1".into() }) }
    async fn create_invoice<'a>(&'a self, _rider_id: String, _amount_cents: i32, _currency: String) -> Result<gen::SmartCityRpcApicreateInvoiceResult, xidl_jsonrpc::Error> { Ok(gen::SmartCityRpcApicreateInvoiceResult { invoice_id: "inv-1".into(), payment_url: "http://pay".into(), r#return: "inv-1".into() }) }
    async fn mark_paid<'a>(&'a self, _invoice_id: String) -> Result<(), xidl_jsonrpc::Error> { Ok(()) }
    async fn heartbeat<'a>(&'a self) -> Result<(), xidl_jsonrpc::Error> { Ok(()) }
    async fn rotate_session<'a>(&'a self, _session_token: String) -> Result<gen::SmartCityRpcApirotateSessionResult, xidl_jsonrpc::Error> { Ok(gen::SmartCityRpcApirotateSessionResult { session_token: "new-tok".into(), expires_at_epoch_sec: 3600 }) }
    async fn dispatch<'a>(&'a self, _vehicle_id: String, _pickup_stop: String) -> Result<gen::SmartCityRpcApidispatchResult, xidl_jsonrpc::Error> { Ok(gen::SmartCityRpcApidispatchResult { job_id: "job-1".into(), r#return: 42 }) }
    async fn report_trip<'a>(&'a self, _order_id: String, _rider_id: String, _status: String) -> Result<(), xidl_jsonrpc::Error> { Ok(()) }
    async fn summarize<'a>(&'a self, _day: String) -> Result<gen::SmartCityRpcApisummarizeResult, xidl_jsonrpc::Error> { Ok(gen::SmartCityRpcApisummarizeResult { trip_count: 10, gross_revenue_cents: 1000 }) }
    async fn get_attribute_region<'a>(&'a self) -> Result<String, xidl_jsonrpc::Error> { Ok("us-east".into()) }
    async fn get_attribute_firmware_channel<'a>(&'a self) -> Result<String, xidl_jsonrpc::Error> { Ok("stable".into()) }
    async fn set_attribute_firmware_channel<'a>(&'a self, _firmware_channel: String) -> Result<(), xidl_jsonrpc::Error> { Ok(()) }
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());
    let server = xidl_jsonrpc::Server::builder().with_service(gen::SmartCityRpcApiServer::new(MySmartCity)).with_endpoint(&format!("tcp://127.0.0.1:{}", port)).build().await?;
    server.serve().await?; Ok(())
}
