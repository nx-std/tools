//! Implementation of the _netloader_ server discovery protocol.
//!
//! The _netloader_ server discovery protocol is used to discover the _netloader_ server in the
//! network using UDP broadcast messages.
//!
//! The client sends a broadcast message to the network to discover the server. The server responds
//! to the broadcast message with the same message. The client listens for the response and
//! determines the IP address of the server.

use std::{
    net::{IpAddr, Ipv4Addr, SocketAddrV4},
    time::Duration,
};

use tokio::{
    io,
    net::{ToSocketAddrs, UdpSocket},
};

use crate::{CLIENT_PORT, SERVER_PORT};

/// The discovery message to send.
///
/// If the _netloader_ was compiled with `PING_ENABLED`, the server will be listening on UDP port
/// `28280` for this message.
const PING_MESSAGE: &[u8] = b"nxboot";

/// The discovery message response to receive.
///
/// The _netloader_ server responds to the discovery message with this message.
const PONG_MESSAGE: &[u8] = b"bootnx";

/// The broadcast address to send the discovery message.
///
/// The _netloader_ server listens on UDP port `28280` for the discovery message.
const BROADCAST_ADDR: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::BROADCAST, SERVER_PORT);

/// The address to bind for receiving the discovery response.
///
/// The _netloader_ server responds to the discovery message on UDP port `28771`.
const RECEIVE_ADDR: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, CLIENT_PORT);

/// Discover the _netloader_ server in the network.
///
/// Broadcasts a discovery message over UDP and waits up to `timeout` for a reply,
/// repeating up to `retries` times. `Ok(None)` means every attempt elapsed without
/// an answer, which is the ordinary outcome when no console is listening.
///
/// # Errors
///
/// Returns an error if either socket cannot be bound — binding the receive socket
/// fails when another process already holds the reply port — if broadcast mode
/// cannot be enabled, or if the final attempt fails to send or receive. A failure
/// on an earlier attempt is retried rather than reported.
pub async fn discover(timeout: Duration, retries: u32) -> Result<Option<IpAddr>, DiscoveryError> {
    let broadcast_socket = UdpSocket::bind("0.0.0.0:0")
        .await
        .map_err(DiscoveryError::BindBroadcastSocket)?;
    broadcast_socket
        .set_broadcast(true)
        .map_err(DiscoveryError::EnableBroadcast)?;

    let receive_socket = UdpSocket::bind(RECEIVE_ADDR)
        .await
        .map_err(DiscoveryError::BindReceiveSocket)?;

    for attempt in 0..retries {
        let ping_fut = async {
            tracing::debug!(attempt, "sending ping message");
            send_ping_message(&broadcast_socket, BROADCAST_ADDR)
                .await
                .map_err(DiscoveryError::SendPing)?;

            tracing::debug!(attempt, "waiting pong response");
            recv_pong_response(&receive_socket).await
        };

        // Run the ping future with a timeout
        match tokio::time::timeout(timeout, ping_fut).await {
            Ok(res) => match res {
                Ok(ip_addr) => {
                    return Ok(Some(ip_addr));
                }
                // The last attempt's failure is the one the caller sees; the console is
                // either absent or unreachable and there is nothing left to try.
                Err(err) if attempt + 1 == retries => {
                    return Err(err);
                }
                // A send or receive failure on an earlier attempt is discarded: discovery
                // is a broadcast on an unreliable transport, so a dropped datagram or a
                // console that has not finished booting is the expected case, not a fault.
                Err(_) => continue,
            },
            // The timeout elapsed with no reply. It carries no information beyond "no
            // console answered within `timeout`", which is what the next attempt retests.
            Err(_) => continue,
        }
    }

    Ok(None)
}

/// Send the discovery ping message to the target address.
async fn send_ping_message<A: ToSocketAddrs>(socket: &UdpSocket, target: A) -> io::Result<()> {
    socket.send_to(PING_MESSAGE, target).await?;
    Ok(())
}

/// Receive the discovery pong message (ping response) from the server.
///
/// # Errors
///
/// Returns an error if the socket cannot be read, or if the datagram that arrived
/// is not the expected reply — the receive port is bound to any address, so an
/// unrelated sender can deliver one.
async fn recv_pong_response(socket: &UdpSocket) -> Result<IpAddr, DiscoveryError> {
    let mut buf = [0u8; 0x10];
    let (len, addr) = socket
        .recv_from(&mut buf)
        .await
        .map_err(DiscoveryError::RecvPong)?;

    if len >= PING_MESSAGE.len() && &buf[0..PONG_MESSAGE.len()] == PONG_MESSAGE {
        Ok(addr.ip())
    } else {
        Err(DiscoveryError::InvalidResponse {
            message: String::from_utf8_lossy(&buf[..len]).into_owned(),
        })
    }
}

/// An error that occurred during the discovery process.
#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    /// The socket used to broadcast the discovery message could not be bound.
    #[error("failed to bind the discovery broadcast socket")]
    BindBroadcastSocket(#[source] io::Error),
    /// Broadcast mode could not be enabled on the sending socket.
    ///
    /// Without it the discovery message reaches nothing, so this is fatal rather
    /// than retried.
    #[error("failed to enable broadcast mode on the discovery socket")]
    EnableBroadcast(#[source] io::Error),
    /// The socket that receives the reply could not be bound.
    ///
    /// The reply port is fixed by the protocol, so this usually means another
    /// process already holds it.
    #[error("failed to bind the discovery reply socket")]
    BindReceiveSocket(#[source] io::Error),
    /// The final attempt could not send the discovery message.
    #[error("failed to send the discovery message")]
    SendPing(#[source] io::Error),
    /// The final attempt could not read the reply socket.
    #[error("failed to receive a datagram on the discovery reply port")]
    RecvPong(#[source] io::Error),
    /// The datagram that arrived is not the expected reply.
    ///
    /// The reply port is bound to any address, so an unrelated sender can deliver
    /// one. Holds the payload rendered lossily, since it need not be UTF-8.
    #[error("unexpected reply on the discovery port: '{message}'")]
    InvalidResponse {
        /// The payload that arrived, rendered lossily.
        message: String,
    },
}
