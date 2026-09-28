//! Bounded QUIC ingress over an existing UDP socket.
//!
//! The owner reads the public socket once, splits GRO batches, classifies each
//! datagram, then feeds the selected endpoint. Endpoints send directly through
//! that same socket; no relay sockets or forwarding tasks are created here.

use std::{
    future::poll_fn,
    io::{self, IoSliceMut},
    net::SocketAddr,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};

use quinn::{
    udp::{RecvMeta, Transmit},
    AsyncUdpSocket, Runtime, UdpPoller,
};
use tokio::sync::mpsc;

pub const SHARED_QUIC_QUEUE_CAPACITY: usize = 128;
pub const MAX_SHARED_DATAGRAM_BYTES: usize = 65_535;

/// One public socket with a single receive owner and independent send pollers.
#[derive(Clone, Debug)]
pub struct SharedUdpSocket {
    socket: Arc<dyn AsyncUdpSocket>,
}

impl SharedUdpSocket {
    /// Wrap an already bound socket. Cloning a descriptor does not bind a port.
    pub fn new(socket: &std::net::UdpSocket) -> io::Result<Self> {
        Ok(Self {
            socket: quinn::TokioRuntime.wrap_udp_socket(socket.try_clone()?)?,
        })
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.socket.local_addr()
    }

    /// Receive a batch with its original address, ECN, destination and stride.
    /// Exactly one owner must call this; endpoint adapters never read this socket.
    pub async fn recv(&self, buffer: &mut [u8]) -> io::Result<RecvMeta> {
        let mut meta = [RecvMeta::default()];
        let mut buffers = [IoSliceMut::new(buffer)];
        poll_fn(|cx| self.socket.poll_recv(cx, &mut buffers, &mut meta)).await?;
        Ok(meta[0])
    }

    /// Create a bounded ingress queue for one QUIC endpoint.
    pub fn endpoint(&self) -> (SharedQuicIngress, Arc<dyn AsyncUdpSocket>) {
        let (sender, receiver) = mpsc::channel(SHARED_QUIC_QUEUE_CAPACITY);
        (
            SharedQuicIngress { sender },
            Arc::new(SharedQuicSocket {
                socket: Arc::clone(&self.socket),
                receiver: Mutex::new(receiver),
            }),
        )
    }
}

#[derive(Debug)]
struct Datagram {
    bytes: Vec<u8>,
    meta: RecvMeta,
}

/// Nonblocking ingress. Full queues drop admission rather than growing memory.
#[derive(Clone, Debug)]
pub struct SharedQuicIngress {
    sender: mpsc::Sender<Datagram>,
}

impl SharedQuicIngress {
    /// Feed exactly one datagram after splitting any GRO batch at `stride`.
    pub fn try_send(&self, bytes: &[u8], meta: RecvMeta) -> io::Result<()> {
        if bytes.is_empty()
            || bytes.len() > MAX_SHARED_DATAGRAM_BYTES
            || meta.len != bytes.len()
            || meta.stride != bytes.len()
            || meta.addr.port() == 0
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid shared QUIC datagram",
            ));
        }
        // Reserve before allocating: rejected traffic never allocates a payload.
        let permit = self.sender.try_reserve().map_err(|error| match error {
            mpsc::error::TrySendError::Full(_) => io::Error::from(io::ErrorKind::WouldBlock),
            mpsc::error::TrySendError::Closed(_) => io::Error::from(io::ErrorKind::BrokenPipe),
        })?;
        permit.send(Datagram {
            bytes: bytes.to_vec(),
            meta,
        });
        Ok(())
    }
}

#[derive(Debug)]
struct SharedQuicSocket {
    socket: Arc<dyn AsyncUdpSocket>,
    receiver: Mutex<mpsc::Receiver<Datagram>>,
}

impl AsyncUdpSocket for SharedQuicSocket {
    fn create_io_poller(self: Arc<Self>) -> Pin<Box<dyn UdpPoller>> {
        Arc::clone(&self.socket).create_io_poller()
    }

    fn try_send(&self, transmit: &Transmit<'_>) -> io::Result<()> {
        self.socket.try_send(transmit)
    }

    fn poll_recv(
        &self,
        cx: &mut Context<'_>,
        buffers: &mut [IoSliceMut<'_>],
        meta: &mut [RecvMeta],
    ) -> Poll<io::Result<usize>> {
        if buffers.is_empty() || meta.is_empty() {
            return Poll::Ready(Err(io::Error::from(io::ErrorKind::InvalidInput)));
        }
        let mut receiver = match self.receiver.lock() {
            Ok(receiver) => receiver,
            Err(_) => {
                return Poll::Ready(Err(io::Error::other("shared QUIC ingress lock poisoned")))
            }
        };
        match receiver.poll_recv(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(None) => Poll::Ready(Err(io::Error::from(io::ErrorKind::BrokenPipe))),
            Poll::Ready(Some(packet)) => {
                if packet.bytes.len() > buffers[0].len() {
                    return Poll::Ready(Err(io::Error::from(io::ErrorKind::InvalidInput)));
                }
                buffers[0][..packet.bytes.len()].copy_from_slice(&packet.bytes);
                meta[0] = packet.meta;
                Poll::Ready(Ok(1))
            }
        }
    }

    fn local_addr(&self) -> io::Result<SocketAddr> {
        self.socket.local_addr()
    }
    fn max_transmit_segments(&self) -> usize {
        self.socket.max_transmit_segments()
    }
    fn may_fragment(&self) -> bool {
        self.socket.may_fragment()
    }
}

#[cfg(test)]
#[path = "shared_udp_tests.rs"]
mod tests;
