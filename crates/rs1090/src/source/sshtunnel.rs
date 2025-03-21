use bytes::BytesMut;
use futures::future::BoxFuture;
use futures::pin_mut;
use makiko::{Tunnel, TunnelReceiver};
use std::future::Future;
use std::io::{Error, ErrorKind};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite};

pub struct SshTunnelIo {
    /// The part of the tunnel used for sending data
    /// needs to be Arc-ed for when it is part of the BoxFuture below
    tunnel: Arc<Tunnel>,
    /// The listening part of the tunnel for receiving data
    tunnel_rx: TunnelReceiver,
    /// When data is received and doesn't fit the buffer, it may be stored here
    pending_read: Option<BytesMut>,
    /// The future for sending data, waiting to be polled
    pending_write_fut: Option<BoxFuture<'static, Result<(), makiko::Error>>>,
    /// The size of the data waiting to be sent, stored to be able to return
    /// Ok(self.pending_write_size)
    pending_write_size: usize,
}

impl SshTunnelIo {
    pub fn new(tunnel: Tunnel, tunnel_rx: TunnelReceiver) -> Self {
        Self {
            tunnel: Arc::new(tunnel),
            tunnel_rx,
            pending_read: None,
            pending_write_fut: None,
            pending_write_size: 0,
        }
    }
    /// The async function needs to have not &mut self as parameters
    async fn send_in_tunnel(
        tunnel: Arc<Tunnel>,
        buf: Vec<u8>,
    ) -> Result<(), makiko::Error> {
        tunnel.send_data(buf.into()).await
    }

    /// A wrapper to send data and store the future in the structure
    fn send(&mut self, buf: &[u8]) -> usize {
        self.pending_write_fut = Some(Box::pin(Self::send_in_tunnel(
            self.tunnel.clone(),
            buf.to_vec(),
        )));
        buf.len()
    }
}

impl AsyncWrite for SshTunnelIo {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        let this = self.get_mut();
        if this.pending_write_fut.is_none() {
            this.pending_write_size = this.send(buf);
            // do not reply now, monitor the status
        }

        match this.pending_write_fut.as_mut().unwrap().as_mut().poll(cx) {
            Poll::Pending => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Poll::Ready(Ok(())) => {
                let _ = this.pending_write_fut.take();
                // do not send anything new yet, but we can say sending is done
                Poll::Ready(Ok(this.pending_write_size))
            }
            Poll::Ready(Err(err)) => {
                let _ = this.pending_write_fut.take();
                Poll::Ready(Err(Error::new(ErrorKind::Other, err)))
            }
        }
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        // The tunnel does not require flushing.
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        // Optional: if your tunnel API supports shutdown (e.g. sending an EOF),
        // call it here. Otherwise, simply signal that shutdown is complete.
        let send_future = self.tunnel.send_eof();
        pin_mut!(send_future);
        match send_future.poll(_cx) {
            Poll::Pending => {
                _cx.waker().wake_by_ref();
                Poll::Pending
            }
            Poll::Ready(Ok(())) => Poll::Ready(Ok(())),
            Poll::Ready(Err(e)) => {
                Poll::Ready(Err(Error::new(ErrorKind::Other, e)))
            }
        }
    }
}

impl AsyncRead for SshTunnelIo {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let this = self.get_mut();

        // First, check if there is pending data from a previous poll.
        if let Some(mut pending) = this.pending_read.take() {
            let to_copy = std::cmp::min(buf.remaining(), pending.len());
            buf.put_slice(&pending[..to_copy]);
            if to_copy < pending.len() {
                // Save any leftover data back.
                this.pending_read = Some(pending.split_off(to_copy));
            }
            return Poll::Ready(Ok(()));
        }

        // Now that we know there is none, we can listen to new data
        match Pin::new(&mut this.tunnel_rx).poll_recv(cx) {
            Poll::Ready(Ok(Some(event))) => {
                match event {
                    makiko::TunnelEvent::Data(mut data) => {
                        // Copy as many bytes as possible into the provided buffer.
                        let to_copy =
                            std::cmp::min(buf.remaining(), data.len());
                        buf.put_slice(&data[..to_copy]);
                        if to_copy < data.len() {
                            // Save any leftover data back.
                            this.pending_read =
                                Some(data.split_off(to_copy).into());
                        }
                        Poll::Ready(Ok(()))
                    }
                    makiko::TunnelEvent::Eof => {
                        // Signal EOF by not filling any more bytes.
                        Poll::Ready(Ok(()))
                    }
                    _ => {
                        // For any other events, simply continue polling later.
                        cx.waker().wake_by_ref();
                        Poll::Pending
                    }
                }
            }
            Poll::Ready(Ok(None)) => {
                // The stream is finished.
                Poll::Ready(Ok(()))
            }
            Poll::Ready(Err(e)) => {
                // Convert error type as needed.
                Poll::Ready(Err(Error::new(ErrorKind::Other, e)))
            }
            Poll::Pending => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        }
    }
}
