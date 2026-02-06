pub trait AsyncStream: tokio::io::AsyncRead + tokio::io::AsyncWrite {}

impl<T> AsyncStream for T where T: tokio::io::AsyncRead + tokio::io::AsyncWrite {}
