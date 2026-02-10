use tokio::io::{ReadHalf, SimplexStream, WriteHalf};

pub type Writer = WriteHalf<SimplexStream>;
pub type Reader = ReadHalf<SimplexStream>;

pub fn pipe() -> Result<(Writer, Reader), std::io::Error> {
    let (rx, tx) = tokio::io::simplex(8192);
    Ok((tx, rx))
}
