use anyhow::Result;
use hyper::client::conn::Builder;
use hyper::Body;
use hyper::Request;
use http::Uri;
use tokio::net::UnixStream;
use tracing::error;


/// Makes an HTTP request over a Unix Domain Socket
pub async fn make_uds_request(
    socket_path: &str,
    request: Request<Body>,
) -> Result<hyper::Response<Body>> {
    // Connect to the UDS socket
    let stream = UnixStream::connect(socket_path).await?;

    // Build the HTTP connection
    let (mut sender, connection) = Builder::new()
        .handshake::<UnixStream, Body>(stream)
        .await?;

    // Spawn the connection task
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            error!("HTTP connection error: {}", e);
        }
    });

    // Send the request
    let response = sender.send_request(request).await?;

    Ok(response)
}

/// Creates a hyper request with the proper URI for UDS communication
pub fn create_uds_uri(path: &str) -> Result<Uri> {
    let uri_str = format!("http://litellm{}", path);
    uri_str.parse().map_err(|e| anyhow::anyhow!("Invalid URI: {}", e))
}