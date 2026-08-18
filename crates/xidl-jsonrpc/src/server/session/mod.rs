mod buffered_stream;
mod response;
#[cfg(test)]
mod tests;

use crate::codec::Codec;
use crate::{Error, Handler, JSONRPC_VERSION, RpcResponse};
use serde_json::Value;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite, BufReader, BufWriter, ReadHalf, WriteHalf};
use tokio::sync::{Mutex, OwnedSemaphorePermit, Semaphore};
use tokio::task::JoinSet;

use buffered_stream::BufferedStream;
use response::ResponseCodec;

const DEFAULT_MAX_IN_FLIGHT_PER_CONNECTION: usize = 64;

/// A parsed JSON-RPC request with the id separated from the payload.
struct ParsedRequest {
    id: Option<Value>,
    method: String,
    params: Value,
}

/// A request that failed structural validation.
struct InvalidRequest {
    id: Option<Value>,
    reason: &'static str,
}

pub(crate) struct ServerSession<RW, H> {
    reader: Option<BufReader<ReadHalf<RW>>>,
    writer: Option<Arc<Mutex<BufWriter<WriteHalf<RW>>>>>,
    handler: Arc<H>,
    codec: Codec,
    local_in_flight: Arc<Semaphore>,
    global_in_flight: Arc<Semaphore>,
    tasks: JoinSet<Result<(), Error>>,
}

impl<RW, H> ServerSession<RW, H>
where
    H: Handler + 'static,
    RW: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    #[cfg(test)]
    pub(crate) fn with_codec(stream: RW, handler: H, codec: Codec) -> Self {
        Self::with_limits(stream, handler, codec, Arc::new(Semaphore::new(256)))
    }

    pub(crate) fn with_limits(
        stream: RW,
        handler: H,
        codec: Codec,
        global_in_flight: Arc<Semaphore>,
    ) -> Self {
        let (read_half, write_half) = tokio::io::split(stream);
        Self {
            reader: Some(BufReader::new(read_half)),
            writer: Some(Arc::new(Mutex::new(BufWriter::new(write_half)))),
            handler: Arc::new(handler),
            codec,
            local_in_flight: Arc::new(Semaphore::new(DEFAULT_MAX_IN_FLIGHT_PER_CONNECTION)),
            global_in_flight,
            tasks: JoinSet::new(),
        }
    }

    pub(crate) async fn run(&mut self) -> Result<(), Error> {
        loop {
            self.reap_finished()?;
            let Some(reader) = self.reader.as_mut() else {
                break;
            };
            let read_result = self.codec.read::<_, Value>(reader).await;
            let value = match read_result {
                Ok(Some(value)) => value,
                Ok(None) => break,
                Err(err) if Self::is_decode_error(&err) => {
                    self.write_error(None, err).await?;
                    continue;
                }
                Err(err) => return Err(err),
            };
            let continue_loop = match value {
                Value::Array(items) => self.spawn_batch(items).await?,
                other => self.handle_value(other).await?,
            };
            if !continue_loop {
                break;
            }
        }
        self.await_tasks().await
    }

    fn is_decode_error(error: &Error) -> bool {
        match error {
            Error::Json(_) => true,
            #[cfg(feature = "msgpack")]
            Error::Msgpack(_) => true,
            Error::FrameTooLarge { .. } => true,
            _ => false,
        }
    }

    fn parse_request(value: Value) -> Result<ParsedRequest, InvalidRequest> {
        let Value::Object(mut map) = value else {
            return Err(InvalidRequest {
                id: None,
                reason: "request must be an object",
            });
        };
        let version_ok = map
            .get("jsonrpc")
            .is_some_and(|value| value.as_str() == Some(JSONRPC_VERSION));
        if !version_ok {
            return Err(InvalidRequest {
                id: map.get("id").cloned(),
                reason: "missing or invalid jsonrpc version",
            });
        }
        let Some(method) = map
            .remove("method")
            .and_then(|value| value.as_str().map(str::to_string))
        else {
            return Err(InvalidRequest {
                id: map.get("id").cloned(),
                reason: "missing method",
            });
        };
        let id = map.remove("id");
        let params = map.remove("params").unwrap_or(Value::Null);
        Ok(ParsedRequest { id, method, params })
    }

    async fn handle_value(&mut self, value: Value) -> Result<bool, Error> {
        match Self::parse_request(value) {
            Ok(request) => self.handle_request(request).await,
            Err(invalid) => {
                self.write_error(invalid.id, Error::Protocol(invalid.reason))
                    .await?;
                Ok(true)
            }
        }
    }

    async fn spawn_batch(&mut self, items: Vec<Value>) -> Result<bool, Error> {
        if items.is_empty() {
            self.write_error(None, Error::Protocol("empty batch"))
                .await?;
            return Ok(true);
        }
        let (global, local) =
            Self::acquire_permits(self.global_in_flight.clone(), self.local_in_flight.clone())
                .await?;
        let Some(writer) = self.writer.as_ref().cloned() else {
            return Err(Error::Protocol("missing stream"));
        };
        let handler = self.handler.clone();
        let codec = self.codec;
        self.tasks.spawn(async move {
            let _permits = (global, local);
            let responses = Self::process_batch(handler, items).await;
            if responses.is_empty() {
                return Ok(());
            }
            Self::write_responses_to(&writer, codec, responses).await
        });
        Ok(true)
    }

    #[cfg(test)]
    async fn handle_batch(&mut self, items: Vec<Value>) -> Result<bool, Error> {
        if items.is_empty() {
            self.write_error(None, Error::Protocol("empty batch"))
                .await?;
            return Ok(true);
        }
        let responses = Self::process_batch(self.handler.clone(), items).await;
        if !responses.is_empty() {
            self.write_responses(responses).await?;
        }
        Ok(true)
    }

    async fn process_batch(handler: Arc<H>, items: Vec<Value>) -> Vec<RpcResponse> {
        let mut responses = Vec::new();
        if items.is_empty() {
            responses.push(ResponseCodec::error(None, Error::Protocol("empty batch")));
            return responses;
        }
        for item in items {
            let request = match Self::parse_request(item) {
                Ok(request) => request,
                Err(invalid) => {
                    responses.push(ResponseCodec::error(
                        invalid.id,
                        Error::Protocol(invalid.reason),
                    ));
                    continue;
                }
            };
            if handler.accepts_bidi(&request.method) {
                if request.id.is_some() {
                    responses.push(ResponseCodec::error(
                        request.id,
                        Error::Protocol("bidi method not allowed in batch"),
                    ));
                }
                continue;
            }
            let response = match handler.handle(&request.method, request.params).await {
                Ok(value) => ResponseCodec::success(request.id, value),
                Err(err) => ResponseCodec::error(request.id, err),
            };
            if response.id.is_some() {
                responses.push(response);
            }
        }
        responses
    }

    async fn handle_request(&mut self, request: ParsedRequest) -> Result<bool, Error> {
        if self.handler.accepts_bidi(&request.method) {
            if let Err(error) = self
                .handler
                .validate_bidi(&request.method, &request.params)
                .await
            {
                if request.id.is_some() {
                    self.write_error(request.id, error).await?;
                }
                return Ok(true);
            }
            self.await_tasks().await?;
            if request.id.is_some() {
                self.write_result(request.id, Value::Null).await?;
            }
            let stream = self.take_stream().await?;
            let bidi = crate::stream::open_bidi_server_with(stream, self.codec);
            self.handler
                .handle_bidi(&request.method, request.params, bidi)
                .await?;
            return Ok(false);
        }

        let (global, local) =
            Self::acquire_permits(self.global_in_flight.clone(), self.local_in_flight.clone())
                .await?;
        let Some(writer) = self.writer.as_ref().cloned() else {
            return Err(Error::Protocol("missing stream"));
        };
        let handler = self.handler.clone();
        let codec = self.codec;
        self.tasks.spawn(async move {
            let _permits = (global, local);
            let response = match handler.handle(&request.method, request.params).await {
                Ok(value) => ResponseCodec::success(request.id, value),
                Err(error) => ResponseCodec::error(request.id, error),
            };
            if response.id.is_some() {
                Self::write_response_to(&writer, codec, response).await?;
            }
            Ok(())
        });
        Ok(true)
    }

    async fn acquire_permits(
        global_in_flight: Arc<Semaphore>,
        local_in_flight: Arc<Semaphore>,
    ) -> Result<(OwnedSemaphorePermit, OwnedSemaphorePermit), Error> {
        let local = local_in_flight
            .acquire_owned()
            .await
            .map_err(|_| Error::Protocol("server connection in-flight limit closed"))?;
        let global = global_in_flight
            .acquire_owned()
            .await
            .map_err(|_| Error::Protocol("server global in-flight limit closed"))?;
        Ok((global, local))
    }

    async fn take_stream(&mut self) -> Result<BufferedStream<RW>, Error> {
        let reader = self
            .reader
            .take()
            .ok_or(Error::Protocol("missing stream"))?;
        let buffered = reader.buffer().to_vec();
        let reader = reader.into_inner();
        let writer = self
            .writer
            .take()
            .ok_or(Error::Protocol("missing stream"))?;
        let writer = Arc::try_unwrap(writer)
            .map_err(|_| Error::Protocol("stream still has active request writers"))?
            .into_inner();
        let write_half = writer.into_inner();
        Ok(BufferedStream::new(reader.unsplit(write_half), buffered))
    }

    fn reap_finished(&mut self) -> Result<(), Error> {
        while let Some(result) = self.tasks.try_join_next() {
            result.map_err(|_| Error::Protocol("server request task failed"))??;
        }
        Ok(())
    }

    async fn await_tasks(&mut self) -> Result<(), Error> {
        while let Some(result) = self.tasks.join_next().await {
            result.map_err(|_| Error::Protocol("server request task failed"))??;
        }
        Ok(())
    }

    async fn write_result(&mut self, id: Option<Value>, result: Value) -> Result<(), Error> {
        self.write_response(ResponseCodec::success(id, result))
            .await
    }

    async fn write_error(&mut self, id: Option<Value>, error: Error) -> Result<(), Error> {
        self.write_response(ResponseCodec::error(id, error)).await
    }

    async fn write_response(&mut self, response: RpcResponse) -> Result<(), Error> {
        let writer = self
            .writer
            .as_ref()
            .cloned()
            .ok_or(Error::Protocol("missing stream"))?;
        Self::write_response_to(&writer, self.codec, response).await
    }

    #[cfg(test)]
    async fn write_responses(&mut self, responses: Vec<RpcResponse>) -> Result<(), Error> {
        let writer = self
            .writer
            .as_ref()
            .cloned()
            .ok_or(Error::Protocol("missing stream"))?;
        Self::write_responses_to(&writer, self.codec, responses).await
    }

    async fn write_response_to(
        stream: &Arc<Mutex<BufWriter<WriteHalf<RW>>>>,
        codec: Codec,
        response: RpcResponse,
    ) -> Result<(), Error> {
        let mut stream = stream.lock().await;
        codec.write(&mut *stream, &response).await
    }

    async fn write_responses_to(
        stream: &Arc<Mutex<BufWriter<WriteHalf<RW>>>>,
        codec: Codec,
        responses: Vec<RpcResponse>,
    ) -> Result<(), Error> {
        let mut stream = stream.lock().await;
        codec.write(&mut *stream, &responses).await
    }
}
