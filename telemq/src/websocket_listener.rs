use crate::{
  authenticator::Authenticator, connection::Connection, control::ControlSender,
  session_state_store::SessionStateStore, stats::StatsSender,
};
use log::{error, info};
use mqtt_packets::v_3_1_1::ControlPacketCodec;
use std::{net::SocketAddr, sync::Arc, time};
use tokio::{spawn, sync::RwLock};
use warp::{self, filters::ws::WebSocket, Filter};

pub struct WebsocketListener;

impl WebsocketListener {
  pub fn bind(
    addr: SocketAddr,
    authenticator: Arc<RwLock<Authenticator>>,
    control_sender: ControlSender,
    stats_sender: StatsSender,
    inactivity_interval: time::Duration,
    state_store: Arc<RwLock<SessionStateStore>>,
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
        )))
        .map(
          |ws: warp::ws::Ws, addr: Option<SocketAddr>, telemq: TeleMQParams| {
            let addr = addr.unwrap().clone();
            // And then our closure will be called when it completes...
            ws.on_upgrade(move |websocket| async move {
              peer_process(
                websocket,
                addr,
                telemq.authenticator,
                telemq.control_sender,
                telemq.stats_sender,
                telemq.inactivity_interval,
                telemq.state_store,
              )
              .await;
            })
          },
        )
        .map(|reply| warp::reply::with_header(reply, "Sec-WebSocket-Protocol", "mqtt"));

      warp::serve(routes).run(addr).await;
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
) {
  info!("new TCP connection from {:?}", addr);

  let connection = match Connection::new_websocket(
    websocket,
    ControlPacketCodec::new(),
    addr,
    control_sender,
    stats_sender,
    authenticator,
    inactivity_interval,
    state_store,
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
}

impl TeleMQParams {
  fn new(
    authenticator: Arc<RwLock<Authenticator>>,
    control_sender: ControlSender,
    stats_sender: StatsSender,
    inactivity_interval: time::Duration,
    state_store: Arc<RwLock<SessionStateStore>>,
  ) -> Self {
    TeleMQParams {
      authenticator,
      control_sender,
      inactivity_interval,
      stats_sender,
      state_store,
    }
  }
}
