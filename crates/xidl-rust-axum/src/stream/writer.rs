use crate::{Error, Result};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

/// Client-side writer for request-streaming endpoints.
pub struct ClientStreamWriter<T, R> {
    tx: Option<mpsc::Sender<Result<T>>>,
    response: Option<JoinHandle<Result<R>>>,
}

impl<T, R> ClientStreamWriter<T, R> {
    /// Creates a new writer from a send channel and response task.
    pub fn new(tx: mpsc::Sender<Result<T>>, response: JoinHandle<Result<R>>) -> Self {
        Self {
            tx: Some(tx),
            response: Some(response),
        }
    }

    /// Sends a stream item to the server.
    pub async fn write(&mut self, item: T) -> Result<()> {
        let tx = self
            .tx
            .as_mut()
            .ok_or_else(|| Error::new(500, "stream writer is already closed"))?;
        tx.send(Ok(item))
            .await
            .map_err(|_| Error::new(500, "stream writer is closed"))
    }

    /// Closes the request stream and awaits the final response.
    pub async fn close(mut self) -> Result<R> {
        let _ = self.tx.take();
        let response = self
            .response
            .take()
            .ok_or_else(|| Error::new(500, "stream writer is already closed"))?;
        response
            .await
            .map_err(|err| Error::new(500, err.to_string()))?
    }

    /// Aborts the request stream without waiting for a response.
    pub async fn cancel(mut self) -> Result<()> {
        let _ = self.tx.take();
        if let Some(response) = self.response.take() {
            response.abort();
        }
        Ok(())
    }
}

impl<T, R> Drop for ClientStreamWriter<T, R> {
    fn drop(&mut self) {
        let _ = self.tx.take();
    }
}
