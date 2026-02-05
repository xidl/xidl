use std::collections::VecDeque;
use std::io::{self, Read, Write};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub struct MemPipe {
    inner: Arc<Mutex<VecDeque<u8>>>,
}

pub type Sender = MemPipe;
pub type Recver = MemPipe;

pub fn pipe() -> io::Result<(Sender, Recver)> {
    let inner = Arc::new(Mutex::new(VecDeque::new()));
    Ok((
        MemPipe {
            inner: inner.clone(),
        },
        MemPipe { inner },
    ))
}

impl Read for MemPipe {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        loop {
            let mut inner = self
                .inner
                .lock()
                .map_err(|_| io::Error::new(io::ErrorKind::Other, "mem pipe lock poisoned"))?;
            if inner.is_empty() {
                drop(inner);
                std::thread::yield_now();
                continue;
            }
            let mut n = 0;
            while n < buf.len() {
                match inner.pop_front() {
                    Some(byte) => {
                        buf[n] = byte;
                        n += 1;
                    }
                    None => break,
                }
            }
            return Ok(n);
        }
    }
}

impl Write for MemPipe {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "mem pipe lock poisoned"))?;
        inner.extend(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
