//! The `cargo nx link` subcommand.
//!
//! This is a Rust implementation of the `nxlink` command-line tool.
//!
//! It sends a file to the Nintendo Switch using the _nx-hbmenu netloader_.
//!
//! See: https://github.com/switchbrew/switch-tools/blob/22756068dd0ed6ff9734c59cb4f99ebd3f62555b/src/nxlink.c

use std::{
    io,
    net::{IpAddr, Ipv4Addr},
    path::PathBuf,
    time::Duration,
};

use nx_netloader::loader::send::send_nro_file;

use crate::ui;

/// Handle the `link` subcommand.
#[tokio::main(flavor = "current_thread")]
pub async fn handle_subcommand(
    Args {
        address,
        retries,
        path,
        extra_args,
        server,
        nro_file,
        mut nro_args,
    }: Args,
) -> Result<(), Error> {
    tracing::debug!(file = %nro_file.display(), "resolving NRO file");

    // Validate the input file
    if !nro_file.exists() {
        return Err(Error::FileNotFound { path: nro_file });
    }
    if !nro_file.is_file() {
        return Err(Error::NotAFile { path: nro_file });
    }
    if nro_file.extension().is_none_or(|ext| ext != "nro") {
        return Err(Error::InvalidExtension { path: nro_file });
    }

    // Get the file name
    let Some(nro_file_name) = nro_file
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
    else {
        return Err(Error::NoFileName { path: nro_file });
    };

    tracing::debug!(file_name = %nro_file_name, "resolved NRO file name");

    // If the path has a `.nro` extension, use it as the destination path.
    // Otherwise, if it ends with a `/`, join the file name onto it.
    let dest_path = match path {
        Some(path) => {
            if path.extension().is_some_and(|ext| ext == "nro") {
                let Some(path_str) = path.to_str() else {
                    return Err(Error::NonUtf8Path { path });
                };
                path_str.to_string()
            } else if path.to_str().is_some_and(|path| path.ends_with('/')) {
                let joined = path.join(&nro_file_name);
                let Some(joined_str) = joined.to_str() else {
                    return Err(Error::NonUtf8Path { path: joined });
                };
                joined_str.to_string()
            } else {
                return Err(Error::InvalidUploadPath { path });
            }
        }
        // Otherwise, use the NRO file name
        None => nro_file_name,
    };

    tracing::debug!(dest = %dest_path, "resolved destination path");

    // Open the file for reading
    let mut file = std::fs::File::open(&nro_file).map_err(|err| Error::OpenFile {
        path: nro_file.clone(),
        source: err,
    })?;

    // Get the file length
    let file_length = file
        .metadata()
        .map_err(|err| Error::FileMetadata {
            path: nro_file,
            source: err,
        })?
        .len() as usize;

    tracing::debug!(length = file_length, "resolved file length");

    // Parse the extra arguments, and add them to the NRO arguments
    if let Some(extra_args) = extra_args {
        let extra_args = parse_extra_args(extra_args);
        if !extra_args.is_empty() {
            nro_args.extend(extra_args);
        }
    }

    // Determine the server IP address
    let remote_addr = match address {
        Some(ip_addr) => (ip_addr, nx_netloader::SERVER_PORT),
        None => {
            let discovered =
                nx_netloader::loader::discovery::discover(Duration::from_millis(250), retries)
                    .await
                    .map_err(Error::Discovery)?;
            let Some(ip_addr) = discovered else {
                return Err(Error::NoServerFound);
            };
            (ip_addr, nx_netloader::SERVER_PORT)
        }
    };

    ui::status("Sending", &format!("{dest_path} to {}", remote_addr.0));

    // Send the file to the remote server
    tokio::select! {biased;
        res = send_nro_file(remote_addr, &dest_path, &mut file, file_length, nro_args) => {
            res.map_err(Error::Send)?;
            ui::status("Finished", "file sent");
        }
        _ = tokio::signal::ctrl_c() => {
            ui::warning("aborted by the user");
            return Ok(());
        }
    }

    // Start the nxlink stdio server if requested
    if server {
        ui::status(
            "Listening",
            "nxlink stdio server started (press Ctrl+C to exit)",
        );

        let stdio_server_addr = (Ipv4Addr::UNSPECIFIED, nx_netloader::CLIENT_PORT);
        tokio::select! {biased;
            _ = nx_netloader::stdio::start_server(stdio_server_addr) => {}
            _ = tokio::signal::ctrl_c() => {}
        }
    }

    Ok(())
}

/// The `link` subcommand CLI arguments.
#[derive(clap::Args)]
pub struct Args {
    /// The IP address of the netloader server.
    #[arg(short, long, value_parser)]
    pub address: Option<IpAddr>,
    /// The number of times to retry server discovery.
    #[arg(short, long, default_value_t = 10)]
    pub retries: u32,
    /// Set upload path for the file.
    #[arg(short, long, value_parser)]
    pub path: Option<PathBuf>,
    /// Extra arguments to pass to the NRO file.
    #[arg(long = "args", value_name = "ARGS")]
    pub extra_args: Option<String>,
    /// Start the nxLink stdio server after a successful file transfer.
    #[arg(short, long, action)]
    pub server: bool,
    /// NRO file to send to the netloader server.
    #[arg(value_name = "FILE", value_parser)]
    pub nro_file: PathBuf,
    /// Args to send to NRO
    #[arg(value_name = "ARGS", value_parser)]
    pub nro_args: Vec<String>,
}

/// Errors from the `link` subcommand.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The NRO file passed on the command line does not exist.
    #[error("File does not exist: '{}'", path.display())]
    FileNotFound { path: PathBuf },

    /// The NRO path passed on the command line is not a regular file.
    #[error("Path is not a file: '{}'", path.display())]
    NotAFile { path: PathBuf },

    /// The input file does not have the required `.nro` extension.
    #[error("File must have a `.nro` extension: '{}'", path.display())]
    InvalidExtension { path: PathBuf },

    /// The input path has no final component to use as a file name.
    #[error("Could not determine the file name of '{}'", path.display())]
    NoFileName { path: PathBuf },

    /// A path could not be represented as UTF-8 for the upload protocol.
    #[error("Path is not valid UTF-8: '{}'", path.display())]
    NonUtf8Path { path: PathBuf },

    /// The `--path` upload target is neither a `.nro` file nor a directory.
    #[error("Invalid upload path: '{}'", path.display())]
    InvalidUploadPath { path: PathBuf },

    /// The input NRO file could not be opened for reading.
    #[error("Failed to open file '{}'", path.display())]
    OpenFile { path: PathBuf, source: io::Error },

    /// The input NRO file's metadata could not be read.
    #[error("Failed to read metadata of '{}'", path.display())]
    FileMetadata { path: PathBuf, source: io::Error },

    /// No netloader server answered discovery on the local network.
    #[error("No netloader server found on the network")]
    NoServerFound,

    /// The netloader server discovery process failed.
    #[error("Server discovery failed")]
    Discovery(#[source] io::Error),

    /// Transferring the NRO file to the netloader server failed.
    #[error("Failed to send the file")]
    Send(#[source] io::Error),
}

impl ui::CliError for Error {}

/// Parse the extra arguments CLI string into a vector of arguments.
fn parse_extra_args(args: String) -> Vec<String> {
    let mut args_chars = args.trim().chars();
    let mut result = Vec::new();

    let mut current_arg = String::new();
    while let Some(current_char) = args_chars.next() {
        if current_char == ' ' {
            continue;
        }

        // If the argument is quoted, parse until the closing quote,
        // otherwise parse until the next space
        if current_char == '"' || current_char == '\'' {
            let quote = current_char;
            for c in args_chars.by_ref() {
                if c == quote {
                    break;
                }
                current_arg.push(c);
            }
        } else {
            // Add the current character to the current argument
            current_arg.push(current_char);

            // Parse until the next space
            for c in args_chars.by_ref() {
                if c == ' ' {
                    break;
                }
                current_arg.push(c);
            }
        }

        // Add the current argument to the result
        if !current_arg.is_empty() {
            result.push(std::mem::take(&mut current_arg));
        }
    }

    result
}
