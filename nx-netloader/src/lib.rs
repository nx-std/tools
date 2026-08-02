//! Host side of the _nxlink_ protocol, for deploying homebrew to a console over
//! the network.
//!
//! The crate speaks to the _netloader_ daemon already running on the console: it
//! finds one by UDP broadcast, sends an NRO to it over TCP, and optionally serves
//! the console's redirected stdout back to the terminal. Nothing here builds or
//! packs an executable — it transfers one that already exists.
//!
//! The two directions use different ports, because the console answers discovery
//! on a port of its own choosing rather than the one it was asked from: see
//! [`SERVER_PORT`] and [`CLIENT_PORT`].

pub mod loader;
pub mod stdio;

/// The _netloader_ server port.
///
/// The _netloader_ server listens on this port for:
/// - _TCP_: Incoming file transfers.
/// - _UDP_: Discovery messages.
pub const SERVER_PORT: u16 = 28280;

/// The _netloader_ client port.
///
/// The server sends the response to the discovery message to this UDP port.
pub const CLIENT_PORT: u16 = 28771;
