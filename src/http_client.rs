use anyhow::{Context, Result};
use reqwest::Client;
use std::path::Path;

/// Create a Unix socket client for local service communication
///
/// Accepts either a raw path or a URI with `unix://` scheme.
///
/// # Arguments
/// * `socket_path` - Path to the Unix socket (with or without `unix://` prefix)
///
/// # Examples
/// ```no_run
/// use omnect_ui::http_client::unix_socket_client;
///
/// // Raw path
/// let client = unix_socket_client("/socket/api.sock")
///     .expect("failed to create client");
///
/// // URI with unix:// scheme
/// let client = unix_socket_client("unix:///socket/api.sock")
///     .expect("failed to create client");
/// ```
pub fn unix_socket_client(socket_path: &str) -> Result<Client> {
    let socket_path = Path::new(socket_path.strip_prefix("unix://").unwrap_or(socket_path));

    // Verify the socket path exists
    if !socket_path
        .try_exists()
        .context("failed to check if socket path exists")?
    {
        anyhow::bail!("failed since socket path does not exist: {socket_path:?}");
    }

    Client::builder()
        .unix_socket(socket_path)
        .build()
        .context("failed to create Unix socket HTTP client")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unix_socket_client_rejects_nonexistent_path() {
        let socket_path = "/tmp/nonexistent-test.sock";
        let result = unix_socket_client(socket_path);
        // Should fail because the socket doesn't exist
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("socket path does not exist")
        );
    }

    #[test]
    fn test_unix_socket_client_rejects_nonexistent_unix_uri() {
        let socket_path = "unix:///tmp/nonexistent-workload.sock";
        let result = unix_socket_client(socket_path);
        // Should strip unix:// prefix and then fail because socket doesn't exist
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("socket path does not exist")
        );
    }
}
