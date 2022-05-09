use async_std::{
    io::{self, Read},
    stream::Stream,
    task::{Context, Poll},
};
use std::pin::Pin;

// See https://github.com/http-rs/tide/issues/852#issuecomment-991638032
#[derive(Debug)]
pub struct BufferedBytesStream<T> {
    pub inner: T,
}

impl<T: Read + Unpin> Stream for BufferedBytesStream<T> {
    type Item = async_std::io::Result<Vec<u8>>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut buf = [0u8; 2048];
        let rd = Pin::new(&mut self.inner);

        match rd.poll_read(cx, &mut buf) {
            Poll::Ready(Ok(0)) => Poll::Ready(None),
            Poll::Ready(Ok(n)) => Poll::Ready(Some(Ok(buf[..n].to_vec()))),
            Poll::Ready(Err(ref e)) if e.kind() == io::ErrorKind::Interrupted => Poll::Pending,
            Poll::Ready(Err(e)) => Poll::Ready(Some(Err(e))),
            _ => Poll::Pending,
        }
    }
}
