use std::{io, net::SocketAddr, sync::Arc, time};

use crate::{
    authenticator::Authenticator,
    config::TeleMQServerConfig,
    connection::Connection,
    control::{Control, ControlMessage, ControlSender},
    server_error::ServerResult,
    session_state_store::SessionStateStore,
    stats::{Stats, StatsSender},
    tls_listener::TlsListener,
    websocket_listener::WebsocketListener,
};

use log::{debug, error, info};
use mqtt_packets::v_3_1_1::ControlPacketCodec;
use signal_hook::{consts::signal::*, low_level::exit};
use signal_hook_tokio::{Handle, Signals};
use tokio::{
    net::{TcpListener, TcpStream},
    select, spawn,
    sync::{
        mpsc::{channel, Receiver, UnboundedSender},
        RwLock,
    },
};
use tokio_rustls::server::TlsStream;
use tokio_stream::StreamExt;
use tokio_util::codec::Framed;

pub struct Server {
    control_sender: ControlSender,
    stats_sender: StatsSender,
    config: TeleMQServerConfig,
    authenticator: Arc<RwLock<Authenticator>>,
    state_store: Arc<RwLock<SessionStateStore>>,
    shut_down_channel: Receiver<()>,
}

impl Server {
    pub fn new(config: TeleMQServerConfig) -> Option<Self> {
        let (tx, rx) = channel(1);
        let state_store = Arc::new(RwLock::new(SessionStateStore::new()));

        let (control, control_sender) = Control::new(&config, state_store.clone(), tx);
        spawn(async move {
            if let Err(err) = control.run().await {
                error!("[Control Worker]: finished with error {:?}", err);
            }
        });

        let (stats, stats_sender) = Stats::new();
        spawn(async move {
            if let Err(err) = stats.run().await {
                error!("[Stats Worker]: finished with error {:?}", err);
            }
        });

        let authenticator = Arc::new(RwLock::new(Authenticator::new(&config).ok()?));

        Some(Server {
            control_sender,
            stats_sender,
            config,
            authenticator,
            state_store,
            shut_down_channel: rx,
        })
    }

    pub async fn start(mut self) -> ServerResult<()> {
        let tcp_listener = TcpListener::bind(&self.config.tcp_addr).await?;
        println!("TCP Listener is listening on {:?}", self.config.tcp_addr);

        let tls_listener = TlsListener::new(
            self.config.tls_addr.clone(),
            &self.config.cert_file,
            &self.config.key_file,
            self.config.keep_alive.clone(),
        )
        .await?;

        if let Some(tls_addr) = self.config.tls_addr {
            println!("TLS Listener is listening on {:?}", tls_addr);
        }

        if let Some(web_addr) = self.config.web_addr {
            WebsocketListener::bind(
                web_addr,
                self.authenticator.clone(),
                self.control_sender.clone(),
                self.stats_sender.clone(),
                self.config.keep_alive.clone(),
                self.state_store.clone(),
            );
            println!("Websocket is listening on {:?}", web_addr);
        }

        let mut signals = Signals::new(&[SIGHUP, SIGTERM, SIGINT, SIGQUIT])?;

        // TODO:
        // if let Some(admin_api_socket) = self.config.admin_api {
        //     let stats = self.stats.clone();
        //     let authenticator = self.authenticator.clone();
        //     spawn(async move {
        //         admin_api::run(admin_api_socket, authenticator, stats).await;
        //     });
        // }

        loop {
            select! {
              Ok((stream, addr)) = tcp_listener.accept() => {
                let authenticator = self.authenticator.clone();
                let control_sender = self.control_sender.clone();
                let stats_sender = self.stats_sender.clone();
                let inactivity_interval = self.config.keep_alive.clone();
                let state_store = self.state_store.clone();
                stream.set_ttl(self.config.keep_alive.as_secs() as u32)?;

                spawn(async move {
                    if let Err(err) = peer_process_tcp(
                        stream,
                        addr,
                        control_sender,
                        stats_sender,
                        authenticator,
                        inactivity_interval,
                        state_store
                    ).await {
                        error!("Could not add new TCP connection: {:?}: {:?}", addr, err);
                    }
                });
              }
              Ok((stream, addr)) = tls_listener.accept() => {
                let control_sender = self.control_sender.clone();
                let stats_sender = self.stats_sender.clone();
                let authenticator = self.authenticator.clone();
                let inactivity_interval = self.config.keep_alive.clone();
                let state_store = self.state_store.clone();

                spawn(async move {
                    if let Err(err) = peer_process_tls(
                        stream,
                        addr,
                        control_sender,
                        stats_sender,
                        authenticator,
                        inactivity_interval,
                        state_store
                    ).await {
                        error!("Could not add new TCP connection: {:?}: {:?}", addr, err);
                    }
                });
              }
              Some(signal) = signals.next() => {
                if handle_os_signal(signal, self.control_sender.clone(), signals.handle()).await? {
                  exit(0);
                } else {
                  debug!("continue");
                }
              }
              Some(_) = self.shut_down_channel.recv() => {
                  println!("[Server Worker]: Shutting down complete. Bye.");
                  signals.handle().close();
                  exit(0);
              }
            }
        }
    }
}

async fn peer_process_tcp(
    stream: TcpStream,
    addr: SocketAddr,
    control_sender: ControlSender,
    stats_sender: StatsSender,
    authenticator: Arc<RwLock<Authenticator>>,
    inactivity_interval: time::Duration,
    state_store: Arc<RwLock<SessionStateStore>>,
) -> ServerResult<()> {
    let packets = Framed::new(stream, ControlPacketCodec::new());

    let connection = Connection::new_tcp(
        packets,
        addr,
        control_sender,
        stats_sender,
        authenticator,
        inactivity_interval,
        state_store,
    )
    .await
    .map_err(|err| format!("{:?}", err))?;

    connection.run().await.map_err(Into::into)
}

async fn peer_process_tls(
    stream: TlsStream<TcpStream>,
    addr: SocketAddr,
    control_sender: ControlSender,
    stats_sender: StatsSender,
    authenticator: Arc<RwLock<Authenticator>>,
    inactivity_interval: time::Duration,
    state_store: Arc<RwLock<SessionStateStore>>,
) -> ServerResult<()> {
    let packets = Framed::new(stream, ControlPacketCodec::new());

    let connection = Connection::new_tls(
        packets,
        addr,
        control_sender,
        stats_sender,
        authenticator,
        inactivity_interval,
        state_store,
    )
    .await
    .map_err(|err| format!("{:?}", err))?;

    connection.run().await.map_err(Into::into)
}

async fn handle_os_signal(
    signal: i32,
    control_sender: UnboundedSender<ControlMessage>,
    handle: Handle,
) -> io::Result<bool> {
    match signal {
        SIGHUP => {
            // reload configuation
            info!("reload configuration");
            Ok(false)
        }
        SIGQUIT => {
            handle.close();
            Ok(true)
        }
        signal if signal == SIGTERM || signal == SIGINT => {
            info!("Shuting down TeleMQ... Please wait, it can take some time");
            control_sender
                .send(ControlMessage::ShutDown)
                .map_err(|err| {
                    io::Error::new(
                        io::ErrorKind::Other,
                        format!("[Server]: unable to gracefully shut down TeleMQ {:?}", err),
                    )
                })?;
            // handle.close();
            Ok(false)
        }
        _ => unreachable!(),
    }
}
