//! Send an NRO file to the _netloader_ server.
//!
//! This module provides functions to send an NRO file to the _netloader_ server. The server will
//! save the file with the specified name if available space permits and will execute the file
//! afterward.

use std::{
    io,
    io::{BufReader, Read},
};

use flate2::{Compression, bufread::ZlibEncoder};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::{TcpStream, ToSocketAddrs},
};

/// The maximum file chunk size to compress and send to the server.
const MAX_FILE_CHUNK_SIZE: usize = 0x4000;

/// The maximum NRO command-line arguments buffer size.
const MAX_CMD_BUF_SIZE: usize = 3072;

/// Send a file to the _netloader_ server.
///
/// This function sends a file to the _netloader_ server at the specified IP address. The server
/// will save the file with `file_name` if available space permits. The file is sent in chunks of
/// compressed data using the _deflate_ algorithm.
///
/// # Errors
///
/// Returns an error if the console cannot be reached, if it rejects the transfer
/// after being told the name and size, if the file cannot be read or compressed,
/// or if a write fails part-way through. A failure after the name is acknowledged
/// leaves a partial file on the console, which the protocol offers no way to
/// withdraw.
pub async fn send_nro_file<A: ToSocketAddrs, R: Read>(
    dst: A,
    file_name: &str,
    file_reader: &mut R,
    file_length: usize,
    cmd_args: impl AsRef<[String]>,
) -> Result<(), SendNroError> {
    let mut sock = TcpStream::connect(dst)
        .await
        .map_err(SendNroError::Connect)?;
    send_file_name_and_length(&mut sock, file_name, file_length).await?;
    compress_and_send_nro_file_data(&mut sock, file_reader, file_length).await?;
    send_nro_args(&mut sock, cmd_args).await?;
    Ok(())
}

/// Errors that can occur when sending a NRO file to the _netloader_ server.
#[derive(Debug, thiserror::Error)]
pub enum SendNroError {
    /// The console could not be reached on the transfer port.
    #[error("failed to connect to the netloader server")]
    Connect(#[source] io::Error),

    /// The file name or its length could not be written.
    #[error("failed to send the file name and length")]
    SendFileName(#[source] io::Error),

    /// The console's answer to the file name could not be read.
    #[error("failed to read the acknowledgement of the file name")]
    ReadNameAck(#[source] io::Error),

    /// The console could not create the destination file.
    ///
    /// Refused after being told the name and size, so nothing has been written on
    /// the console yet.
    #[error("the netloader server could not create the file")]
    CouldNotCreateFile,

    /// The console has insufficient space for the file.
    ///
    /// Refused after being told the name and size, so nothing has been written on
    /// the console yet.
    #[error("the netloader server has insufficient space for the file")]
    InsufficientSpace,

    /// The console does not recognize the file's extension.
    ///
    /// Refused after being told the name and size, so nothing has been written on
    /// the console yet.
    #[error("the netloader server did not recognize the file extension")]
    FileExtensionNotRecognized,

    /// The console refused the file with a code this crate does not know.
    #[error("the netloader server refused the file (code {code})")]
    RefusedWithUnknownCode {
        /// The code the console replied with.
        code: i32,
    },

    /// The local file could not be read or compressed.
    #[error("failed to read the file being sent")]
    ReadFile(#[source] io::Error),

    /// A compressed chunk could not be written.
    ///
    /// The console has already created the file, so it is left partially written.
    #[error("failed to send a chunk of the file")]
    SendChunk(#[source] io::Error),

    /// The console's answer to the transferred data could not be read.
    #[error("failed to read the acknowledgement of the transfer")]
    ReadDataAck(#[source] io::Error),

    /// The console reported a problem with the transferred data.
    ///
    /// The protocol assigns no meaning to the codes at this step, so the raw value
    /// is all there is to report.
    #[error("the netloader server rejected the transferred data (code {code})")]
    DataRejected {
        /// The code the console replied with.
        code: i32,
    },

    /// The command-line arguments could not be written.
    #[error("failed to send the command-line arguments")]
    SendArgs(#[source] io::Error),
}

/// Send the file name and length to the _netloader_ server.
///
/// This function sends the file name and size to the _netloader_ server. The server will respond
/// with an acknowledgement code.
async fn send_file_name_and_length<S>(
    stream: &mut S,
    file_name: &str,
    file_length: usize,
) -> Result<(), SendNroError>
where
    S: AsyncRead + AsyncWrite + Unpin + ?Sized,
{
    // Send the file name (length-prefixed)
    write_length_prefixed(stream, file_name)
        .await
        .map_err(SendNroError::SendFileName)?;

    // Send the file length
    stream
        .write_u32_le(file_length as u32)
        .await
        .map_err(SendNroError::SendFileName)?;

    // Wait and check the acknowledgement code
    let rc = stream
        .read_i32_le()
        .await
        .map_err(SendNroError::ReadNameAck)?;
    match rc {
        0 => Ok(()),
        _ => Err(refusal_from_ack(rc)),
    }
}

/// Map the console's non-zero acknowledgement of the file name to an error.
///
/// Not a `From` impl: only four of [`SendNroError`]'s variants come from a code,
/// and zero is not a refusal at all.
fn refusal_from_ack(code: i32) -> SendNroError {
    debug_assert!(code != 0, "unexpected success code");
    match code {
        -1 => SendNroError::CouldNotCreateFile,
        -2 => SendNroError::InsufficientSpace,
        -3 => SendNroError::FileExtensionNotRecognized,
        _ => SendNroError::RefusedWithUnknownCode { code },
    }
}

/// Send the file content to the _nxlink_ server compressed with the deflate algorithm.
///
/// This function sends the file content to the _nxlink_ server compressed with the deflate
/// algorithm. The server will respond with an acknowledgement code.
async fn compress_and_send_nro_file_data<S, R>(
    stream: &mut S,
    file_reader: &mut R,
    file_length: usize,
) -> Result<(), SendNroError>
where
    S: AsyncRead + AsyncWrite + Unpin + ?Sized,
    R: Read,
{
    let mut encoder = ZlibEncoder::new(BufReader::new(file_reader), Compression::default());

    loop {
        // Read a data chunk from the file
        let mut buf = [0u8; MAX_FILE_CHUNK_SIZE];
        let read_len = encoder.read(&mut buf).map_err(SendNroError::ReadFile)?;
        if read_len == 0 {
            break;
        }

        // Send the compressed data chunk (length-prefixed)
        write_length_prefixed(stream, &buf[..read_len])
            .await
            .map_err(SendNroError::SendChunk)?;

        // Log the progress
        let bytes_sent = encoder.total_in();
        tracing::debug!(
            bytes_sent,
            percent = (bytes_sent as f64 * 100.0) / file_length as f64,
            "sent a compressed chunk"
        );
    }

    // Wait and check the response code
    let rc = stream
        .read_i32_le()
        .await
        .map_err(SendNroError::ReadDataAck)?;
    if rc != 0 {
        return Err(SendNroError::DataRejected { code: rc });
    }

    Ok(())
}

/// Send the NRO command-line arguments to the _nxlink_ server
async fn send_nro_args<S>(stream: &mut S, args: impl AsRef<[String]>) -> Result<(), SendNroError>
where
    S: AsyncWrite + Unpin + ?Sized,
{
    let mut cmd_buf = [0u8; MAX_CMD_BUF_SIZE];
    let mut cmd_buf_len = 0;

    // Write the command-line arguments to the buffer
    for arg in args.as_ref() {
        let arg_bytes = arg.as_bytes();

        // Check if the argument fits in the buffer, otherwise break
        if cmd_buf_len + arg_bytes.len() + 1 > MAX_CMD_BUF_SIZE {
            break;
        }

        // Write the argument to the buffer. The null terminator is already there:
        // the buffer is zeroed, and the guard above reserved a byte for it.
        cmd_buf[cmd_buf_len..cmd_buf_len + arg_bytes.len()].copy_from_slice(arg_bytes);
        cmd_buf_len += arg_bytes.len() + 1;
    }

    // Send the command-line arguments (length-prefixed)
    write_length_prefixed(stream, &cmd_buf[..cmd_buf_len])
        .await
        .map_err(SendNroError::SendArgs)?;

    Ok(())
}

/// Write a length-prefixed data to the stream.
///
/// Writes the length of the data as a `u32` (little-endian) followed by the data bytes to the
/// stream.
async fn write_length_prefixed<S>(stream: &mut S, data: impl AsRef<[u8]>) -> io::Result<()>
where
    S: AsyncWrite + Unpin + ?Sized,
{
    let data = data.as_ref();
    let data_len = data.len() as u32;

    stream.write_u32_le(data_len).await?;
    stream.write_all(data).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{MAX_CMD_BUF_SIZE, send_nro_args};

    /// Run `send_nro_args` into an in-memory stream and return the payload it
    /// wrote, with the `u32` length prefix stripped and checked.
    async fn sent_args_payload(args: &[String]) -> Vec<u8> {
        let mut stream: Vec<u8> = Vec::new();
        send_nro_args(&mut stream, args)
            .await
            .expect("writing into a Vec should succeed");

        let (prefix, payload) = stream.split_at(4);
        let declared = u32::from_le_bytes(
            prefix
                .try_into()
                .expect("a 4-byte slice converts into [u8; 4]"),
        );
        assert_eq!(
            declared as usize,
            payload.len(),
            "the length prefix should match the payload written after it"
        );
        payload.to_vec()
    }

    #[tokio::test]
    async fn send_nro_args_terminates_every_argument_with_a_null() {
        //* Given
        let args = ["sdmc:/hello.nro".to_string(), "--verbose".to_string()];

        //* When
        let payload = sent_args_payload(&args).await;

        //* Then
        assert_eq!(
            payload, b"sdmc:/hello.nro\0--verbose\0",
            "arguments should be concatenated, each null-terminated"
        );
    }

    #[tokio::test]
    async fn send_nro_args_with_no_arguments_sends_an_empty_payload() {
        //* Given
        let args: [String; 0] = [];

        //* When
        let payload = sent_args_payload(&args).await;

        //* Then
        assert!(payload.is_empty(), "no arguments should send no bytes");
    }

    #[tokio::test]
    async fn send_nro_args_drops_an_argument_that_would_overflow_the_buffer() {
        //* Given
        // The second argument cannot fit alongside the first, so it is dropped
        // rather than truncated into a name the console would misread.
        let args = [
            "a".repeat(MAX_CMD_BUF_SIZE - 2),
            "would-not-fit".to_string(),
        ];

        //* When
        let payload = sent_args_payload(&args).await;

        //* Then
        assert_eq!(
            payload.len(),
            MAX_CMD_BUF_SIZE - 1,
            "only the first argument and its terminator should be sent"
        );
        assert_eq!(
            payload.last(),
            Some(&0),
            "the surviving argument should still be terminated"
        );
    }
}
