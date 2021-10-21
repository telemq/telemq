use std::{fs::File, io, net::SocketAddr, path::Path, sync::Arc, time::Duration};

use futures::future::pending;
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::{
    rustls::{
        internal::pemfile::{certs, rsa_private_keys},
        Certificate, NoClientAuth, PrivateKey, ServerConfig,
    },
    server::TlsStream,
    TlsAcceptor,
};

pub struct TlsListener {
    listener: Option<TcpListener>,
    config: Option<ServerConfig>,
    keep_alive: Duration,
}

impl TlsListener {
    pub async fn new(
        maybe_addr: Option<SocketAddr>,
        maybe_cert_path: &Option<String>,
        maybe_key_path: &Option<String>,
        keep_alive: Duration,
    ) -> io::Result<Self> {
        match (maybe_addr, maybe_cert_path, maybe_key_path) {
            (Some(addr), Some(cert_path), Some(key_path)) => {
                let certs = load_certs(Path::new(&cert_path))?;
                let mut keys = load_keys(Path::new(&key_path))?;
                let mut config = ServerConfig::new(NoClientAuth::new());
                config
                    .set_single_cert(certs, keys.remove(0))
                    .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;
                Ok(TlsListener {
                    listener: Some(TcpListener::bind(&addr).await?),
                    config: Some(config),
                    keep_alive,
                })
            }
            _ => Ok(TlsListener {
                listener: None,
                config: None,
                keep_alive,
            }),
        }
    }

    pub async fn accept(&self) -> io::Result<(TlsStream<TcpStream>, SocketAddr)> {
        match (&self.listener, &self.config) {
            (Some(listener), Some(config)) => {
                let (stream, addr) = listener.accept().await?;
                stream.set_ttl(self.keep_alive.as_secs() as u32)?;
                let acceptor = TlsAcceptor::from(Arc::new(config.clone()));
                let stream = acceptor.accept(stream).await?;
                Ok((stream, addr))
            }
            _ => pending().await,
        }
    }
}

fn load_certs(path: &Path) -> io::Result<Vec<Certificate>> {
    certs(&mut io::BufReader::new(File::open(path)?))
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid cert"))
}

fn load_keys(path: &Path) -> io::Result<Vec<PrivateKey>> {
    rsa_private_keys(&mut io::BufReader::new(File::open(path)?))
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid key"))
}
