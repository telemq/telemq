use crate::{
  authenticator::Authenticator, connection::Connection, control::ControlSender,
  session_state_store::SessionStateStore, stats::StatsSender,
};
use log::{error, info};
use mqtt_packets::v_3_1_1::ControlPacketCodec;
use std::{
  net::SocketAddr,
  sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
  },
  time,
};
use tokio::{spawn, sync::RwLock};
use warp::{self, filters::ws::WebSocket, Filter, Reply};

pub struct WssListener;

impl WssListener {
  pub fn bind(
    addr: SocketAddr,
    connections_number: Arc<AtomicUsize>,
    authenticator: Arc<RwLock<Authenticator>>,
    control_sender: ControlSender,
    stats_sender: StatsSender,
    inactivity_interval: time::Duration,
    state_store: Arc<RwLock<SessionStateStore>>,
    max_connections: usize,
    max_subs_per_client: Option<usize>,
    cert_path: String,
    key_path: String,
  ) {
    spawn(async move {
      let routes = warp::ws()
        .and(warp::addr::remote())
        .and(with_telemq(TeleMQParams::new(
          authenticator,
          control_sender,
          stats_sender,
          inactivity_interval,
          state_store,
          connections_number,
          max_connections,
          max_subs_per_client,
        )))
        .map(
          |ws: warp::ws::Ws, addr: Option<SocketAddr>, telemq: TeleMQParams| {
            info!("[WSS Listener Worker] new connection {:?}", addr);
            let addr = addr.unwrap().clone();
            if telemq
              .connections_number
              .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |prev_value| {
                if prev_value >= telemq.max_connections {
                  None
                } else {
                  Some(prev_value + 1)
                }
              })
              .is_err()
            {
              return warp::http::StatusCode::from_u16(560)
                .unwrap()
                .into_response();
            }
            // And then our closure will be called when it completes...
            ws.on_upgrade(move |websocket| async move {
              println!("WSS upgrade");
              peer_process(
                websocket,
                addr,
                telemq.authenticator,
                telemq.control_sender,
                telemq.stats_sender,
                telemq.inactivity_interval,
                telemq.state_store,
                telemq.max_subs_per_client,
              )
              .await;
              telemq.connections_number.fetch_sub(1, Ordering::Relaxed);
            })
            .into_response()
          },
        )
        .map(|reply| warp::reply::with_header(reply, "Sec-WebSocket-Protocol", "mqtt"));
      warp::serve(routes)
        .tls()
        .cert_path(cert_path)
        .key_path(key_path)
        .run(addr)
        .await;
    });
  }
}

async fn peer_process(
  websocket: WebSocket,
  addr: SocketAddr,
  authenticator: Arc<RwLock<Authenticator>>,
  control_sender: ControlSender,
  stats_sender: StatsSender,
  inactivity_interval: time::Duration,
  state_store: Arc<RwLock<SessionStateStore>>,
  max_subs_per_client: Option<usize>,
) {
  info!("new TCP connection from {:?}", addr);

  let connection = match Connection::new_ws(
    websocket,
    ControlPacketCodec::new(),
    addr,
    control_sender,
    stats_sender,
    authenticator,
    inactivity_interval,
    state_store,
    max_subs_per_client,
  )
  .await
  .map_err(|err| format!("{:?}", err))
  {
    Ok(c) => c,
    Err(err) => {
      error!(
        "[Websocket Connection {:?}] could not create connection {:?}",
        addr, err
      );
      return;
    }
  };

  if let Err(err) = connection.run().await {
    error!("[Websocket Connection {:?}] {:?}", addr, err);
  }
}

fn with_telemq(
  telemq: TeleMQParams,
) -> impl Filter<Extract = (TeleMQParams,), Error = std::convert::Infallible> + Clone {
  warp::any().map(move || telemq.clone())
}

#[derive(Clone)]
struct TeleMQParams {
  authenticator: Arc<RwLock<Authenticator>>,
  control_sender: ControlSender,
  inactivity_interval: time::Duration,
  stats_sender: StatsSender,
  state_store: Arc<RwLock<SessionStateStore>>,
  connections_number: Arc<AtomicUsize>,
  max_connections: usize,
  max_subs_per_client: Option<usize>,
}

impl TeleMQParams {
  fn new(
    authenticator: Arc<RwLock<Authenticator>>,
    control_sender: ControlSender,
    stats_sender: StatsSender,
    inactivity_interval: time::Duration,
    state_store: Arc<RwLock<SessionStateStore>>,
    connections_number: Arc<AtomicUsize>,
    max_connections: usize,
    max_subs_per_client: Option<usize>,
  ) -> Self {
    TeleMQParams {
      authenticator,
      control_sender,
      inactivity_interval,
      stats_sender,
      state_store,
      connections_number,
      max_connections,
      max_subs_per_client,
    }
  }
}
