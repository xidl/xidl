/// Trait implemented by generated Axum services.
///
/// A service knows how to convert itself into an [`axum::Router`] that can be
/// merged into a larger application.
pub trait Service: Send + Sync + 'static {
    /// Consumes the service and produces its router.
    fn into_router(self) -> axum::Router;
}
