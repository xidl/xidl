use crate::{Error, ErrorCode, RpcError, RpcRequestOwned, RpcResponse, JSONRPC_VERSION};
use serde_json::Value;
use std::io::{BufRead, Write};

pub trait Handler {
    fn handle(&self, method: &str, params: Value) -> Result<Value, Error>;
}

pub struct Io<R, W> {
    pub reader: R,
    pub writer: W,
}

impl<R, W> Io<R, W> {
    pub fn new(reader: R, writer: W) -> Self {
        Self { reader, writer }
    }
}

struct MultiHandler {
    services: Vec<Box<dyn Handler>>,
}

impl Handler for MultiHandler {
    fn handle(&self, method: &str, params: Value) -> Result<Value, Error> {
        for service in &self.services {
            match service.handle(method, params.clone()) {
                Ok(value) => return Ok(value),
                Err(err) => {
                    if err.is_method_not_found() {
                        continue;
                    }
                    return Err(err);
                }
            }
        }
        Err(Error::method_not_found(method))
    }
}

pub struct ServerBuilder {
    io: Option<Io<Box<dyn BufRead>, Box<dyn Write>>>,
    services: Vec<Box<dyn Handler>>,
}

pub struct Server {
    io: Io<Box<dyn BufRead>, Box<dyn Write>>,
    services: Vec<Box<dyn Handler>>,
}

impl Server {
    pub fn builder() -> ServerBuilder {
        ServerBuilder {
            io: None,
            services: Vec::new(),
        }
    }

    pub fn serve(self) -> Result<(), Error> {
        let handler = MultiHandler {
            services: self.services,
        };
        serve(self.io.reader, self.io.writer, handler)
    }
}

impl ServerBuilder {
    pub fn with_io<R, W>(mut self, io: Io<R, W>) -> Self
    where
        R: BufRead + 'static,
        W: Write + 'static,
    {
        self.io = Some(Io::new(Box::new(io.reader), Box::new(io.writer)));
        self
    }

    pub fn with_service<S>(mut self, service: S) -> Self
    where
        S: Handler + 'static,
    {
        self.services.push(Box::new(service));
        self
    }

    pub fn serve(self) -> Result<(), Error> {
        let io = self.io.ok_or(Error::Protocol("missing io"))?;
        let server = Server {
            io,
            services: self.services,
        };
        server.serve()
    }
}

pub fn serve<R, W, H>(mut reader: R, mut writer: W, handler: H) -> Result<(), Error>
where
    R: BufRead,
    W: Write,
    H: Handler,
{
    let mut session = ServerSession::new(&mut reader, &mut writer, handler);
    session.run()
}

struct ServerSession<R, W, H> {
    reader: R,
    writer: W,
    handler: H,
}

impl<R, W, H> ServerSession<R, W, H>
where
    R: BufRead,
    W: Write,
    H: Handler,
{
    fn new(reader: R, writer: W, handler: H) -> Self {
        Self {
            reader,
            writer,
            handler,
        }
    }

    fn run(&mut self) -> Result<(), Error> {
        let mut line = String::new();
        loop {
            line.clear();
            let bytes = self.reader.read_line(&mut line)?;
            if bytes == 0 {
                break;
            }
            self.handle_line(&line)?;
        }
        Ok(())
    }

    fn handle_line(&mut self, line: &str) -> Result<(), Error> {
        let request: RpcRequestOwned = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(err) => {
                self.write_error(None, Error::Json(err))?;
                return Ok(());
            }
        };
        let id = request.id;
        let method = match request.method {
            Some(method) => method,
            None => {
                self.write_error(id, Error::Protocol("missing method"))?;
                return Ok(());
            }
        };
        let params = request.params.unwrap_or(Value::Null);

        match self.handler.handle(&method, params) {
            Ok(value) => self.write_result(id, value),
            Err(err) => self.write_error(id, err),
        }
    }

    fn write_result(&mut self, id: Option<u64>, result: Value) -> Result<(), Error> {
        let response = RpcResponse {
            jsonrpc: Some(JSONRPC_VERSION.to_string()),
            id,
            result: Some(result),
            error: None,
        };
        self.write_response(response)
    }

    fn write_error(&mut self, id: Option<u64>, error: Error) -> Result<(), Error> {
        let rpc_error = match error {
            Error::Rpc {
                code,
                message,
                data,
            } => RpcError {
                code: code.code(),
                message,
                data,
            },
            Error::Json(err) => RpcError {
                code: ErrorCode::ParseError.code(),
                message: err.to_string(),
                data: None,
            },
            Error::Protocol(message) => RpcError {
                code: ErrorCode::InvalidRequest.code(),
                message: message.to_string(),
                data: None,
            },
            Error::Io(err) => RpcError {
                code: ErrorCode::InternalError.code(),
                message: err.to_string(),
                data: None,
            },
        };
        let response = RpcResponse {
            jsonrpc: Some(JSONRPC_VERSION.to_string()),
            id,
            result: None,
            error: Some(rpc_error),
        };
        self.write_response(response)
    }

    fn write_response(&mut self, response: RpcResponse) -> Result<(), Error> {
        let payload = serde_json::to_string(&response)?;
        self.writer.write_all(payload.as_bytes())?;
        self.writer.write_all(b"\n")?;
        Ok(())
    }
}
