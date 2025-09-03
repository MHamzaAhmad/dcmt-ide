use wtransport::{ServerConfig, RecvStream};
use wtransport::tls::{Certificate, CertificateChain, PrivateKey, Identity};
use anyhow::Result;

use crate::types::WebTransportMessage;

pub async fn read_message(stream: &mut RecvStream) -> Result<WebTransportMessage> {
    // use tokio::io::AsyncReadExt;
    
    let mut length_bytes = [0u8; 4];
    stream.read_exact(&mut length_bytes).await?;
    let length = u32::from_be_bytes(length_bytes) as usize;
    
    let mut message_bytes = vec![0u8; length];
    stream.read_exact(&mut message_bytes).await?;
    
    let message: WebTransportMessage = bincode::deserialize(&message_bytes)?;
    Ok(message)
}

pub async fn create_server_config(cert_path: &str, key_path: &str) -> Result<ServerConfig> {
    // Load certificate and private key
    let cert_chain = load_cert_chain(cert_path).await?;
    let private_key = load_private_key(key_path).await?;
    
    // Create certificates from raw bytes
    let certs: Result<Vec<Certificate>, _> = cert_chain.into_iter()
        .map(|der| Certificate::from_der(der))
        .collect();
    let certs = certs?;
    
    // Create certificate chain and private key
    let cert_chain = CertificateChain::new(certs);
    let private_key = PrivateKey::from_der_pkcs8(private_key);
    let identity = Identity::new(cert_chain, private_key);
    
    let config = ServerConfig::builder()
        .with_bind_address(([0, 0, 0, 0], 3001).into())
        .with_identity(identity)
        .build();
    
    Ok(config)
}

async fn load_cert_chain(path: &str) -> Result<Vec<Vec<u8>>> {
    use rustls_pemfile;
    
    let cert_file = tokio::fs::read(path).await?;
    let mut reader = std::io::Cursor::new(cert_file);
    
    let certs = rustls_pemfile::certs(&mut reader)
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|cert| cert.to_vec())
        .collect();
        
    Ok(certs)
}

async fn load_private_key(path: &str) -> Result<Vec<u8>> {
    use rustls_pemfile;
    
    let key_file = tokio::fs::read(path).await?;
    let mut reader = std::io::Cursor::new(key_file);
    
    let keys = rustls_pemfile::pkcs8_private_keys(&mut reader)
        .collect::<Result<Vec<_>, _>>()?;
    
    match keys.into_iter().next() {
        Some(key) => Ok(key.secret_pkcs8_der().to_vec()),
        None => Err(anyhow::anyhow!("No private key found in file")),
    }
}