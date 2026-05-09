//! Packaging primitives shared between `cmd::build` and `cmd::bundle`.
//!
//! Each submodule wraps an `nx-object` builder with an in-memory, side-effect-free
//! API: callers pass already-loaded bytes in and receive the serialized output bytes
//! back. Filesystem I/O is the caller's responsibility.

pub mod nacp;
pub mod npdm;
pub mod nro;
pub mod nso;
pub mod nsp;
