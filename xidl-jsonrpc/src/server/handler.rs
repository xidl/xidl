use crate::Error;
use serde_json::Value;
use std::sync::Arc;

#[async_trait::async_trait]
pub trait Handler: Send + Sync {
    async fn handle(&self, method: &str, params: Value) -> Result<Value, Error>;

    fn accepts_bidi(&self, _method: &str) -> bool {
        false
    }

    async fn handle_bidi(
        &self,
        method: &str,
        _params: Value,
        _stream: crate::stream::ReaderWriter<Value, Value>,
    ) -> Result<(), Error> {
        Err(Error::method_not_found(method))
    }
}

#[async_trait::async_trait]
impl<T> Handler for Arc<T>
where
    T: Handler,
{
    async fn handle(&self, method: &str, params: Value) -> Result<Value, Error> {
        (**self).handle(method, params).await
    }

    fn accepts_bidi(&self, method: &str) -> bool {
        (**self).accepts_bidi(method)
    }

    async fn handle_bidi(
        &self,
        method: &str,
        params: Value,
        stream: crate::stream::ReaderWriter<Value, Value>,
    ) -> Result<(), Error> {
        (**self).handle_bidi(method, params, stream).await
    }
}

pub(crate) struct MultiHandler {
    services: Vec<Box<dyn Handler>>,
}

impl MultiHandler {
    pub(crate) fn new(services: Vec<Box<dyn Handler>>) -> Self {
        Self { services }
    }

    async fn dispatch(&self, method: &str, params: Value) -> Result<Value, Error> {
        for service in &self.services {
            match service.handle(method, params.clone()).await {
                Ok(value) => return Ok(value),
                Err(err) if err.is_method_not_found() => continue,
                Err(err) => return Err(err),
            }
        }
        Err(Error::method_not_found(method))
    }

    fn bidi_service(&self, method: &str) -> Option<&dyn Handler> {
        self.services
            .iter()
            .find(|service| service.accepts_bidi(method))
            .map(|service| service.as_ref())
    }
}

#[async_trait::async_trait]
impl Handler for MultiHandler {
    async fn handle(&self, method: &str, params: Value) -> Result<Value, Error> {
        self.dispatch(method, params).await
    }

    fn accepts_bidi(&self, method: &str) -> bool {
        self.bidi_service(method).is_some()
    }

    async fn handle_bidi(
        &self,
        method: &str,
        params: Value,
        stream: crate::stream::ReaderWriter<Value, Value>,
    ) -> Result<(), Error> {
        if let Some(service) = self.bidi_service(method) {
            return service.handle_bidi(method, params, stream).await;
        }
        Err(Error::method_not_found(method))
    }
}

#[cfg(test)]
mod test;
