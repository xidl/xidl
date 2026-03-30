include!(concat!(env!("OUT_DIR"), "/hysteria2.rs"));

pub struct ImHysteria2Server;

#[async_trait::async_trait]
impl Hysteria2 for ImHysteria2Server {
    async fn auth(
        &self,
        req: xidl_rust_axum::Request<Hysteria2AuthRequest>,
    ) -> Result<Hysteria2AuthResponse, xidl_rust_axum::Error> {
        let req = req.into_inner();

        Ok(Hysteria2AuthResponse {
            udp: true,
            rx: req.rx,
            padding: req.padding,
        })
    }
}
