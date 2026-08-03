//! The _nxlink stdio_ server implementation.
//!
//! When the NRO app enables the _nxlink stdio_ feature, it redirects its stdout and stderr
//! streams over TCP.
//!
//! This allows the NRO app to write to a remote console.

use tokio::{
    io,
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, ToSocketAddrs},
};

/// Start the _nxlink stdio_ server.
///
/// Accepts a single connection and forwards it until the console closes it, then
/// returns. It does not serve a second one.
///
/// Deliberately has no deadline, unlike the transfer path: the console connects
/// when the deployed title starts, which is whenever the user gets to it, and a
/// running title may then print nothing for as long as it likes. Both waits are
/// bounded by the caller instead — `cargo nx link` races this against `Ctrl-C`.
///
/// <div class="warning">
/// The _nxlink stdio_ runtime on the console expects a TCP server listening at port `28771`.
/// </div>
///
/// # Errors
///
/// Returns an error if the port cannot be bound, if accepting the connection fails,
/// or if reading from it does.
pub async fn start_server<A: ToSocketAddrs>(addr: A) -> io::Result<()> {
    let listener = TcpListener::bind(&addr).await?;
    let (stream, _) = listener.accept().await?;

    tracing::debug!("connection accepted from {}", stream.peer_addr()?);
    handle_stream(stream).await
}

/// Redirect the TCP stream to the Stdout stream.
async fn handle_stream<S>(mut stream: S) -> io::Result<()>
where
    S: AsyncRead + Unpin,
{
    let mut buffer = [0u8; 1024];
    loop {
        match stream.read(&mut buffer).await {
            Ok(0) => {
                tracing::debug!("connection closed");
                break;
            }
            Ok(len) => {
                io::stdout().write_all(&buffer[..len]).await?;
            }
            Err(err) => return Err(err),
        }
    }
    Ok(())
}
