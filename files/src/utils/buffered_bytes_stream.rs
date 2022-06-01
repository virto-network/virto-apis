use async_std::{
    stream::Stream,
    task::{Context, Poll},
};
use futures::io::AsyncRead;
use std::pin::Pin;

// See https://github.com/http-rs/http-types/issues/126#issuecomment-636499905
pub struct BufferedBytesStream<R>(R);

impl<R> BufferedBytesStream<R> {
    pub fn new(reader: R) -> Self {
        Self(reader)
    }
}

impl<R: Unpin + AsyncRead> Stream for BufferedBytesStream<R> {
    type Item = Result<Vec<u8>, futures::io::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let mut buf = [0u8; 1024];
        match Pin::new(&mut self.0).poll_read(cx, &mut buf[..]) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(0)) => Poll::Ready(None),
            Poll::Ready(Ok(n)) => Poll::Ready(Some(Ok(buf[..n].to_vec()))),
            Poll::Ready(Err(e)) => Poll::Ready(Some(Err(e))),
        }
    }
}
