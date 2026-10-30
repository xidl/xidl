use crate::{Error, Result};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

/// Client-side writer for request-streaming endpoints.
pub struct ClientStreamWriter<T, R, E = Error> {
    tx: Option<mpsc::Sender<Result<T>>>,
    response: Option<JoinHandle<std::result::Result<R, E>>>,
}

impl<T, R, E: From<Error>> ClientStreamWriter<T, R, E> {
    /// Creates a new writer from a send channel and response task.
    pub fn new(
        tx: mpsc::Sender<Result<T>>,
        response: JoinHandle<std::result::Result<R, E>>,
    ) -> Self {
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
    pub async fn close(mut self) -> std::result::Result<R, E> {
        let _ = self.tx.take();
        let response = self
            .response
            .take()
            .ok_or_else(|| Error::new(500, "stream writer is already closed"))?;
        response
            .await
            .map_err(|err| E::from(Error::new(500, format!("stream response task: {err}"))))?
    }

    /// Aborts the request stream without waiting for a response.
    pub async fn cancel(mut self) -> Result<()> {
        let _ = self.tx.take();
        if let Some(response) = self.response.take() {
            response.abort();
            if let Err(error) = response.await
                && !error.is_cancelled()
            {
                return Err(Error::message(format!("stream cancellation: {error}")));
            }
        }
        Ok(())
    }
}

impl<T, R, E> Drop for ClientStreamWriter<T, R, E> {
    fn drop(&mut self) {
        let _ = self.tx.take();
        if let Some(response) = self.response.take() {
            response.abort();
        }
    }
}
