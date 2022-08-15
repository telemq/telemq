use std::{
    io,
    net::SocketAddr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time,
};

use crate::{
    authenticator::Authenticator,
    config::TeleMQServerConfig,
    connection::Connection,
    control::{Control, ControlMessage, ControlSender},
    server_error::ServerResult,
    session_state_store::SessionStateStore,
    stats::{Stats, StatsConfig, StatsSender},
    tls_listener::TlsListener,
    ws_listener::WsListener,
    wss_listener::WssListener,
};

use ipnet::IpNet;
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
    connections_number: Arc<AtomicUsize>,
}

impl Server {
    pub async fn new(config: TeleMQServerConfig) -> Option<Self> {
        let (tx, rx) = channel(1);
        let state_store = Arc::new(RwLock::new(SessionStateStore::new()));

        let (control, control_sender) = Control::new(&config, state_store.clone(), tx).await;
        spawn(async move {
            if let Err(err) = control.run().await {
                error!("[Control Worker]: finished with error {:?}", err);
            }
        });

        let (stats, stats_sender) = Stats::new(StatsConfig {
            update_interval: config.sys_topics_update_interval,
            control_sender: control_sender.clone(),
        });
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
            connections_number: Arc::new(AtomicUsize::new(0)),
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

        if let Some(web_addr) = self.config.ws_addr {
            WsListener::bind(
                web_addr,
                self.connections_number.clone(),
                self.authenticator.clone(),
                self.control_sender.clone(),
                self.stats_sender.clone(),
                self.config.keep_alive.clone(),
                self.state_store.clone(),
                self.config.max_connections,
                self.config.max_subs_per_client,
            );
            println!("Websocket is listening on {:?}", web_addr);
        }

        if let (Some(web_tls_addr), &Some(ref cert_path), &Some(ref key_path)) = (
            self.config.wss_addr,
            &self.config.cert_file,
            &self.config.key_file,
        ) {
            WssListener::bind(
                web_tls_addr,
                self.connections_number.clone(),
                self.authenticator.clone(),
                self.control_sender.clone(),
                self.stats_sender.clone(),
                self.config.keep_alive.clone(),
                self.state_store.clone(),
                self.config.max_connections,
                self.config.max_subs_per_client,
                cert_path.clone(),
                key_path.clone(),
            );
            println!("Websocket TLS is listening on {:?}", web_tls_addr);
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
                let add_ip_net = IpNet::from(addr.ip());
                let ip_allowed = self.config.ip_whitelist
                    .as_ref()
                    .map(|allowed_nets| {
                        return !allowed_nets.is_empty()
                            && allowed_nets.iter()
                                .any(|allowed_net| allowed_net.contains(&add_ip_net))
                    })
                    .unwrap_or(true);
                if !ip_allowed {
                    continue;
                }
                let connections_number = self.connections_number.clone();
                if connections_number.fetch_update(
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                    |prev_value| if prev_value >= self.config.max_connections {
                        None
                    } else {
                        Some(prev_value + 1)
                    }
                ).is_err() {
                    continue;
                }
                let authenticator = self.authenticator.clone();
                let control_sender = self.control_sender.clone();
                let stats_sender = self.stats_sender.clone();
                let inactivity_interval = self.config.keep_alive.clone();
                let state_store = self.state_store.clone();
                let max_subs_per_client = self.config.max_subs_per_client.clone();
                stream.set_ttl(self.config.keep_alive.as_secs() as u32)?;

                spawn(async move {
                    if let Err(err) = peer_process_tcp(
                        stream,
                        addr,
                        control_sender,
                        stats_sender,
                        authenticator,
                        inactivity_interval,
                        state_store,
                        max_subs_per_client
                    ).await {
                        error!("Could not add new TCP connection: {:?}: {:?}", addr, err);
                    }
                    connections_number.fetch_sub(1, Ordering::Relaxed);
                });
              }
              Ok((stream, addr)) = tls_listener.accept() => {
                let connections_number = self.connections_number.clone();
                if connections_number.fetch_update(
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                    |prev_value| if prev_value >= self.config.max_connections {
                        None
                    } else {
                        Some(prev_value + 1)
                    }
                ).is_err() {
                    continue;
                }
                let control_sender = self.control_sender.clone();
                let stats_sender = self.stats_sender.clone();
                let authenticator = self.authenticator.clone();
                let inactivity_interval = self.config.keep_alive.clone();
                let max_subs_per_client = self.config.max_subs_per_client.clone();
                let state_store = self.state_store.clone();

                spawn(async move {
                    if let Err(err) = peer_process_tls(
                        stream,
                        addr,
                        control_sender,
                        stats_sender,
                        authenticator,
                        inactivity_interval,
                        state_store,
                        max_subs_per_client
                    ).await {
                        error!("Could not add new TCP connection: {:?}: {:?}", addr, err);
                    }
                    connections_number.fetch_sub(1, Ordering::SeqCst);
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
    max_subs_per_client: Option<usize>,
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
        max_subs_per_client,
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
    max_subs_per_client: Option<usize>,
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
        max_subs_per_client,
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
            Ok(false)
        }
        _ => unreachable!(),
    }
}
