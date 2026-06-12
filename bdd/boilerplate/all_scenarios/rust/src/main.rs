use async_trait::async_trait;
mod gen { include!("../{{MODULE_NAME}}.rs"); }
struct MyAllScenarios { status: std::sync::Mutex<gen::Status> }
#[async_trait]
impl gen::AllScenariosService for MyAllScenarios {
    async fn get_item<'a>(&'a self, id: u32, filter: String, trace_id: String) -> Result<String, xidl_rust_axum::Error> { Ok(format!("Item {id} with {filter} and {trace_id}")) }
    async fn create_item<'a>(&'a self, _name: String, _payload: gen::Payload) -> Result<u32, xidl_rust_axum::Error> { Ok(42) }
    async fn update_item<'a>(&'a self, _id: u32, _metadata: Vec<gen::Metadata>) -> Result<(), xidl_rust_axum::Error> { Ok(()) }
    async fn delete_item<'a>(&'a self, _id: u32) -> Result<(), xidl_rust_axum::Error> { Ok(()) }
    async fn get_attribute_system_status(&self) -> Result<gen::Status, xidl_rust_axum::Error> { Ok(*self.status.lock().unwrap()) }
    async fn set_attribute_system_status(&self, value: gen::Status) -> Result<(), xidl_rust_axum::Error> { *self.status.lock().unwrap() = value; Ok(()) }
    async fn get_attribute_version(&self) -> Result<String, xidl_rust_axum::Error> { Ok("1.0.0".into()) }
    async fn upload_form<'a>(&'a self, _key: String, _value: String) -> Result<(), xidl_rust_axum::Error> { Ok(()) }
    async fn secure_data<'a>(&'a self, _auth: xidl_rust_axum::auth::bearer::BearerAuth) -> Result<String, xidl_rust_axum::Error> { Ok("Secret".into()) }
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let svc = gen::AllScenariosServiceServer::new(MyAllScenarios { status: std::sync::Mutex::new(gen::Status::ACTIVE) });
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());
    xidl_rust_axum::Server::builder().with_service(svc).serve(&format!("127.0.0.1:{}", port)).await?; Ok(())
}
