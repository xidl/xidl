include!(concat!(env!("OUT_DIR"), "/hysteria2.rs"));

pub struct ImHysteria2Server;

#[async_trait::async_trait]
impl Hysteria2 for ImHysteria2Server {
    async fn auth(
        &self,
        _auth: String,
        rx: u32,
        padding: String,
    ) -> Result<Hysteria2AuthResponse, xidl_rust_axum::Error> {
        Ok(Hysteria2AuthResponse {
            udp: true,
            rx,
            padding,
        })
    }
}
