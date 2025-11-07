use omnect_ui::http_client::unix_socket_client;
use serde::Serialize;
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::sync::oneshot;

// Integration tests for unix_socket_client
async fn start_mock_unix_socket_server(
    socket_path: PathBuf,
    ready_tx: oneshot::Sender<()>,
) -> std::io::Result<()> {
    let listener = UnixListener::bind(&socket_path)?;

    // Signal that the server is ready
    let _ = ready_tx.send(());

    loop {
        let (mut stream, _) = listener.accept().await?;

        tokio::spawn(async move {
            let mut reader = BufReader::new(&mut stream);
            let mut _headers = Vec::new();

            // Read HTTP headers
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).await.is_err() {
                    return;
                }

                if line.trim().is_empty() {
                    break;
                }

                _headers.push(line);
            }

            // Simple mock response
            let response_body = r#"{"status":"ok","message":"test response"}"#;
            let http_response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                response_body.len(),
                response_body
            );

            let _ = stream.write_all(http_response.as_bytes()).await;
        });
    }
}

#[tokio::test]
async fn test_unix_socket_client_integration_success() {
    // Create a temporary directory for the Unix socket
    let temp_dir = TempDir::new().expect("failed to create temp directory");
    let socket_path = temp_dir.path().join("test.sock");
    let socket_path_clone = socket_path.clone();

    // Create a oneshot channel for server ready signal
    let (ready_tx, ready_rx) = oneshot::channel();

    // Start the mock server in the background
    let server_handle = tokio::spawn(async move {
        let _ = start_mock_unix_socket_server(socket_path_clone, ready_tx).await;
    });

    // Wait for the server to be ready
    ready_rx.await.expect("server failed to start");

    // Create the unix socket client
    let client = unix_socket_client(socket_path.to_str().expect("invalid socket path"))
        .expect("failed to create unix socket client");

    // Make a request to the mock server
    let url = "http://localhost/test";
    let response = client
        .get(url)
        .send()
        .await
        .expect("failed to send request");

    // Verify the response
    assert!(response.status().is_success());

    let body = response.text().await.expect("failed to read response body");
    assert!(body.contains("test response"));

    // Clean up
    server_handle.abort();
}

#[tokio::test]
async fn test_unix_socket_client_integration_post_request() {
    // Create a temporary directory for the Unix socket
    let temp_dir = TempDir::new().expect("failed to create temp directory");
    let socket_path = temp_dir.path().join("test-post.sock");
    let socket_path_clone = socket_path.clone();

    // Create a oneshot channel for server ready signal
    let (ready_tx, ready_rx) = oneshot::channel();

    // Start the mock server in the background
    let server_handle = tokio::spawn(async move {
        let _ = start_mock_unix_socket_server(socket_path_clone, ready_tx).await;
    });

    // Wait for the server to be ready
    ready_rx.await.expect("server failed to start");

    // Create the unix socket client
    let client = unix_socket_client(socket_path.to_str().expect("invalid socket path"))
        .expect("failed to create unix socket client");

    // Make a POST request with JSON payload
    #[derive(Serialize)]
    struct TestPayload {
        name: String,
        value: i32,
    }

    let payload = TestPayload {
        name: "test".to_string(),
        value: 42,
    };

    let url = "http://localhost/api/data";
    let response = client
        .post(url)
        .json(&payload)
        .send()
        .await
        .expect("failed to send request");

    // Verify the response
    assert!(response.status().is_success());

    // Clean up
    server_handle.abort();
}

#[tokio::test]
async fn test_unix_socket_client_integration_multiple_requests() {
    // Create a temporary directory for the Unix socket
    let temp_dir = TempDir::new().expect("failed to create temp directory");
    let socket_path = temp_dir.path().join("test-multi.sock");
    let socket_path_clone = socket_path.clone();

    // Create a oneshot channel for server ready signal
    let (ready_tx, ready_rx) = oneshot::channel();

    // Start the mock server in the background
    let server_handle = tokio::spawn(async move {
        let _ = start_mock_unix_socket_server(socket_path_clone, ready_tx).await;
    });

    // Wait for the server to be ready
    ready_rx.await.expect("server failed to start");

    // Create the unix socket client
    let client = unix_socket_client(socket_path.to_str().expect("invalid socket path"))
        .expect("failed to create unix socket client");

    // Make multiple requests to ensure the client can be reused
    for i in 0..3 {
        let url = format!("http://localhost/test/{}", i);
        let response = client
            .get(&url)
            .send()
            .await
            .expect("failed to send request");

        assert!(response.status().is_success());
    }

    // Clean up
    server_handle.abort();
}
