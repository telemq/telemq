use crate::{
  authenticator::{Authenticator, ConnectResponse as AuthenticatorConnectResponse, TopicAccess},
  connection_provider::SessionConnectionProvider,
  control::{ControlMessage, ControlSender},
  net_connection::NetConnection,
  session_state::SessionState,
  session_state_store::SessionStateStore,
  stats::{StatsMessage, StatsSender},
  transaction::TransactionSendState,
};
// FIXME: define logging levels
use log::{error, info};
use mqtt_packets::v_3_1_1::{
  builders::{
    ConnackBuilder, PingrespPacketBuilder, PubackPacketBuilder, PubcompPacketBuilder,
    PublishPacketBuilder, PubrecPacketBuilder, PubrelPacketBuilder, SubackPacketBuilder,
    UnsubackPacketBuilder,
  },
  connack::return_code::ReturnCode as ConnackReturnCode,
  publish::fixed_header::{get_qos_level, set_qos_level},
  suback::return_code::ReturnCode as SubackReturnCode,
  subscribe::topic_subscription::TopicSubscription,
  topic::{topics_match, Subscription, Topic},
  unsubscribe::variable::Variable as UnsubscribeVariable,
  utils::getters_setters,
  variable::Variable,
  CPType, ControlPacket, ControlPacketCodec, QoS,
};
use std::{io, net::SocketAddr, sync::Arc, time};
use tokio::{
  net::TcpStream,
  select,
  sync::{
    mpsc::{channel, unbounded_channel, Receiver, Sender, UnboundedReceiver, UnboundedSender},
    RwLock,
  },
  time::sleep,
};
use tokio_rustls::server::TlsStream;
use tokio_util::codec::Framed;
use warp::ws::WebSocket;

macro_rules! id {
  ($self: expr) => {
    $self
      .state
      .get_client_id()
      .unwrap_or_else(|_| "UNKNOWN".into())
  };
}

macro_rules! send_control {
  ($control_message: expr, $self: expr) => {
    let message_type = $control_message.get_name();
    if let Err(err) = $self.control_sender.send($control_message) {
      error!(
        "[Connection Worker@{:?}]: unable to send {}. {:?}",
        $self.addr, message_type, err
      );
    }
  };
}

macro_rules! send_stats {
  ($stat_message: expr, $self: expr) => {
    let message_type = $stat_message.get_name();
    if let Err(err) = $self.stats_sender.send($stat_message) {
      error!(
        "[Connection Worker@{:?}]: unable to send {}. {:?}",
        $self.addr, message_type, err
      );
    }
  };
}

macro_rules! disconnect {
  ($self: expr) => {{
    if $self.state.is_connected() {
      send_control!(
        ControlMessage::ClientDisconnected {
          addr: $self.addr.clone(),
          clean_session: $self.state.has_clean_session(),
          client_id: id!($self),
          will_packet: $self.state.get_will_data().map(|will_data| {
            PublishPacketBuilder::new()
              .with_retained(will_data.3)
              .with_qos(&will_data.1)
              .with_topic(will_data.0)
              .with_payload(will_data.2)
              .produce()
          })
        },
        $self
      );
    }
    if let Err(err) = $self.disconnect.0.send(()).await {
      error!(
        "[Connection Worker@{:?}]: Unable to close connection. {:?}",
        $self.addr, err
      );
    }
  }};
}

macro_rules! send {
  ($package: expr, $self: expr) => {
    if $self.packets.send_packet($package).await.is_err() {
      error!(
        "[Connection Worker@{:?}]: Unable to send message, disconnecting",
        $self.addr
      );
      Err::<(), ()>(())
    } else {
      send_stats!(
        StatsMessage::new_packet_processed(id!($self), &$package),
        $self
      );

      Ok::<(), ()>(())
    }
  };
}

macro_rules! send_or_disconnect {
  ($package: expr, $self: expr) => {
    if (send!($package, $self)).is_err() {
      error!("Unable to send message, disconnecting");
      disconnect!($self);
    }
  };
}

pub type ConnectionSender = UnboundedSender<ConnectionMessage>;
pub type ConnectionReceiver = UnboundedReceiver<ConnectionMessage>;

#[derive(Debug)]
pub enum ConnectionMessage {
  Publish {
    packet: ControlPacket,
    retained_for: Option<String>,
  },
  // disconnect a single client (when a new client with the same id has connected)
  Disconnect,
  // will be sent during the whole server shut down
  ShutDown,
}

impl ConnectionMessage {
  pub fn get_name(&self) -> String {
    match self {
      ConnectionMessage::Publish { .. } => "ConnectionMessage::Publish".into(),
      ConnectionMessage::Disconnect => "ConnectionMessage::Disconnect".into(),
      ConnectionMessage::ShutDown => "ConnectionMessage::ShutDown".into(),
    }
  }
}

pub struct Connection {
  addr: SocketAddr,
  pub packets: NetConnection,
  self_sender: Option<ConnectionSender>,
  message_receiver: ConnectionReceiver,
  state: SessionState,
  last_activity: time::Instant,
  authenticator: Arc<RwLock<Authenticator>>,
  disconnect: (Sender<()>, Receiver<()>),
  control_sender: ControlSender,
  stats_sender: StatsSender,
  inactivity_interval: time::Duration,
  acl: Option<AuthenticatorConnectResponse>,
  state_store: Arc<RwLock<SessionStateStore>>,
}

impl Connection {
  pub async fn new_tcp(
    framed: Framed<TcpStream, ControlPacketCodec>,
    addr: SocketAddr,
    control_sender: ControlSender,
    stats_sender: StatsSender,
    authenticator: Arc<RwLock<Authenticator>>,
    inactivity_interval: time::Duration,
    state_store: Arc<RwLock<SessionStateStore>>,
  ) -> io::Result<Self> {
    let (tx_self, rx_self) = unbounded_channel();
    let disconnect = channel(1);

    let state = SessionState::NonConnected;
    let last_activity = time::Instant::now();
    let packets = NetConnection::new_tcp(framed);

    Ok(Connection {
      addr,
      packets,
      message_receiver: rx_self,
      self_sender: Some(tx_self),
      state,
      last_activity,
      authenticator,
      disconnect,
      control_sender,
      stats_sender,
      inactivity_interval,
      acl: None,
      state_store,
    })
  }

  pub async fn new_tls(
    framed: Framed<TlsStream<TcpStream>, ControlPacketCodec>,
    addr: SocketAddr,
    control_sender: ControlSender,
    stats_sender: StatsSender,
    authenticator: Arc<RwLock<Authenticator>>,
    inactivity_interval: time::Duration,
    state_store: Arc<RwLock<SessionStateStore>>,
  ) -> io::Result<Self> {
    let (tx_self, rx_self) = unbounded_channel();

    let state = SessionState::NonConnected;
    let last_activity = time::Instant::now();
    let packets = NetConnection::new_tls(framed);
    let disconnect = channel(1);

    Ok(Connection {
      addr,
      packets,
      message_receiver: rx_self,
      self_sender: Some(tx_self),
      state,
      last_activity,
      authenticator,
      disconnect,
      control_sender,
      stats_sender,
      inactivity_interval,
      acl: None,
      state_store,
    })
  }

  pub async fn new_websocket(
    websocket: WebSocket,
    codec: ControlPacketCodec,
    addr: SocketAddr,
    control_sender: ControlSender,
    stats_sender: StatsSender,
    authenticator: Arc<RwLock<Authenticator>>,
    inactivity_interval: time::Duration,
    state_store: Arc<RwLock<SessionStateStore>>,
  ) -> io::Result<Self> {
    let (tx_self, rx_self) = unbounded_channel();
    let packets = NetConnection::new_web((websocket, codec));
    let disconnect = channel(1);
    let state = SessionState::NonConnected;
    let last_activity = time::Instant::now();

    Ok(Connection {
      addr,
      packets,
      message_receiver: rx_self,
      self_sender: Some(tx_self),
      state,
      last_activity,
      authenticator,
      disconnect,
      control_sender,
      stats_sender,
      inactivity_interval,
      acl: None,
      state_store,
    })
  }
}

impl Connection {
  pub async fn run(mut self) -> io::Result<()> {
    loop {
      select! {
        Some(cmd_message) = self.message_receiver.recv() => {
          match cmd_message {
            ConnectionMessage::Publish{packet, retained_for} => {
              self.forward_publish(packet, retained_for).await;
            }
            ConnectionMessage::Disconnect => {
              info!("[Connection Worker@{:?}]: Disconnecting client. New clinet with the same id connected", self.addr);
              return Ok(());
            },
            ConnectionMessage::ShutDown => {
              self.shut_down().await;
            }
          }
        }
        Some(_) = self.disconnect.1.recv() => {
          info!("[Connection Worker@{:?}]: Disconnecting client. Signal", self.addr);
          return Ok(());
        }
        _ = sleep(self.inactivity_interval) => {
          info!("[Connection Worker@{:?}]: Disconnecting client due to inactivity", self.addr);
          disconnect!(self);
          break;
        }
        res = self.packets.next_packet() => match res {
          Some(Ok(control_packet)) => {
            info!("[Connection Worker@{:?}]: control packet received: {:?}", self.addr, control_packet);
            self.handle_control_packet(control_packet).await;
          },
          Some(Err(err)) => {error!("{:?}", err);}
          None => {
            break;
          }
        }
      }
    }

    if self.state.is_connected() {
      if let Err(err) = self
        .control_sender
        .send(ControlMessage::ClientDisconnected {
          addr: self.addr.clone(),
          clean_session: self.state.has_clean_session(),
          client_id: id!(self),
          will_packet: self.state.get_will_data().map(|will_data| {
            PublishPacketBuilder::new()
              .with_retained(will_data.3)
              .with_qos(&will_data.1)
              .with_topic(will_data.0)
              .with_payload(will_data.2)
              .produce()
          }),
        })
      {
        error!(
          "[Connection Worker@{:?}]: Unable to send ControlMessage::ClientDisconnected. {:?}",
          self.addr, err
        );
      }
    }

    send_stats!(
      StatsMessage::ClientDisconnected {
        client_id: id!(self),
      },
      self
    );

    info!(
      "[Connection Worker@{:?}]: Client has been disconnected",
      self.addr
    );

    Ok(())
  }

  async fn handle_control_packet(&mut self, control_packet: ControlPacket) {
    self.last_activity = time::Instant::now();

    match control_packet.fixed_header.cp_type {
      CPType::Connect => {
        self.connect(control_packet).await;
      }
      CPType::Disconnect => {
        self.disconnect().await;
      }
      CPType::Pingreq => {
        self.pingreq(control_packet).await;
      }
      CPType::Subscribe => {
        self.subscribe(control_packet).await;
      }
      CPType::Unsubscribe => {
        self.unsubscribe(control_packet).await;
      }
      CPType::Publish => {
        self.publish(control_packet).await;
      }
      CPType::Puback => {
        self.puback(&control_packet).await;
      }
      CPType::Pubrec => {
        self.pubrec(&control_packet).await;
      }
      CPType::Pubrel => {
        self.pubrel(&control_packet).await;
      }
      CPType::Pubcomp => {
        self.pubcomp(&control_packet).await;
      }
      CPType::Connack | CPType::Suback | CPType::Pingresp | CPType::Unsuback => {
        // disconnecting a client which sends broker's packets
        error!(
          "[Connection Worker@{:?}] Unexpected packet received from a client. {:?}. Disconnecting",
          self.addr, control_packet
        );
        disconnect!(self);
      }
    }
  }

  // Packet Handlers

  async fn connect(&mut self, mut control_packet: ControlPacket) {
    if !self.state.is_non_connected() {
      error!(
        "[Connection Worker@{:?}]: state is not in non connected state. Unable to connect a client",
        self.addr.clone()
      );
      disconnect!(self);
      return;
    }

    // report stats
    if let Variable::Connect(ref variable) = control_packet.variable {
      send_stats!(
        StatsMessage::new_packet_processed(variable.client_identifier.clone(), &control_packet,),
        self
      );
    }

    if let Variable::Connect(ref mut variable) = control_packet.variable {
      let client_id = variable.client_identifier.clone();
      let clean_session = variable.connect_flags.has_clean_session();

      let allowed_res = self
        .authenticator
        .read()
        .await
        .connect(
          self.addr,
          client_id.clone(),
          variable.username.take(),
          variable.password.take(),
        )
        .await;

      match allowed_res {
        Ok(response) => {
          if !response.connection_allowed {
            let connack = ConnackBuilder::new()
              .with_return_code(ConnackReturnCode::BadUsernameOrPassword)
              .with_session_presented(false)
              .build();
            send_or_disconnect!(&connack, self);
            return;
          }
          self.acl = Some(response);
        }
        Err(err) => {
          error!("[Authenticator Error]: {:?}", err);
          let connack = ConnackBuilder::new()
            .with_return_code(ConnackReturnCode::Unavailable)
            .with_session_presented(false)
            .build();
          send_or_disconnect!(&connack, self);
          return;
        }
      }

      match self.state_store.write().await.take_state(&client_id).await {
        Ok(Some(connected_state)) => {
          if !clean_session {
            info!(
              "[Connection Worker@{:?}]: Recovering saved state\n\t{:?}",
              self.addr, connected_state
            );
            self.state.make_connected(connected_state);
            let connack = ConnackBuilder::new()
              .with_return_code(ConnackReturnCode::Accepted)
              .with_session_presented(true)
              .build();
            send_or_disconnect!(&connack, self);
          } else {
            info!(
              "[Connection Worker@{:?}]: Creating default state",
              self.addr
            );
            self.state.into_connected(SessionConnectionProvider {
              client_id,
              clean_session: variable.connect_flags.has_clean_session(),
              will_topic: variable.will_topic.take(),
              will_message: variable.will_message.take(),
              will_qos: variable.connect_flags.qos_value().ok(),
            });
            let connack = ConnackBuilder::new()
              .with_return_code(ConnackReturnCode::Accepted)
              .with_session_presented(false)
              .build();
            send_or_disconnect!(&connack, self);
          }
        }
        Ok(None) => {
          info!(
            "[Connection Worker@{:?}]: Creating default state",
            self.addr
          );
          self.state.into_connected(SessionConnectionProvider {
            client_id,
            clean_session: variable.connect_flags.has_clean_session(),
            will_topic: variable.will_topic.take(),
            will_message: variable.will_message.take(),
            will_qos: variable.connect_flags.qos_value().ok(),
          });
          let connack = ConnackBuilder::new()
            .with_return_code(ConnackReturnCode::Accepted)
            .with_session_presented(false)
            .build();
          send_or_disconnect!(&connack, self);
        }
        Err(err) => {
          error!(
            "[Connection Worker@{:?}]: Unable to connect with SessionStateStore, falling back to default state. {:?}",
            self.addr, err
          );
          self.state.into_connected(SessionConnectionProvider {
            client_id,
            clean_session,
            will_topic: variable.will_topic.take(),
            will_message: variable.will_message.take(),
            will_qos: variable.connect_flags.qos_value().ok(),
          });
          let connack = ConnackBuilder::new()
            .with_return_code(ConnackReturnCode::Accepted)
            .with_session_presented(false)
            .build();
          send_or_disconnect!(&connack, self);
        }
      }

      if let SessionState::Connected(ref connected) = self.state {
        send_control!(
          ControlMessage::ClientConnected {
            sender: self.self_sender.clone().unwrap(),
            addr: self.addr.clone(),
            client_id: id!(self),
            clean_session: connected.clean_session
          },
          self
        );

        if !connected.subscriptions.is_empty() {
          send_control!(
            ControlMessage::AddSubscriptions {
              addr: self.addr.clone(),
              subscriptions: connected
                .subscriptions
                .iter()
                .map(|(_, s)| s.clone())
                .collect(),
              client_id: id!(self)
            },
            self
          );
        }

        // re-delivery
        let mut sent_successfully = true;
        for (packet_id, transaction) in &connected.messages_sent_not_acked {
          match transaction.state {
            TransactionSendState::NonAcked => {
              // re-publish
              sent_successfully =
                send!(&transaction.control_packet, self).is_ok() & sent_successfully;
            }
            TransactionSendState::PubReced => {
              // re-send Pubrel packet
              let packet = PubrelPacketBuilder::new(packet_id).build();
              sent_successfully = send!(&packet, self).is_ok() & sent_successfully;
            }
            _ => {
              // no required actoin
            }
          }
          if !sent_successfully {
            break;
          }
        }
        if !sent_successfully {
          disconnect!(self);
          return;
        }
      } else {
        // unreachable
        return;
      }

      send_stats!(
        StatsMessage::ClientConnected {
          addr: self.addr.clone(),
          client_id: id!(self)
        },
        self
      );

      for cp in self.state.get_queued_messages() {
        self.forward_publish(cp, None).await;
      }
    } else {
      error!(
        "[Connection Worker@{:?}]: Wrong type of variable. Connect is expected",
        self.addr
      );
      disconnect!(self);
      return;
    }
  }

  async fn disconnect(&mut self) {
    let client_id = id!(self);
    if let Ok(connected_state) = self.state.into_closed() {
      send_stats!(
        StatsMessage::ClientDisconnected {
          client_id: client_id.clone()
        },
        self
      );

      if let Err(err) = self
        .state_store
        .write()
        .await
        .save_state(connected_state)
        .await
      {
        error!(
          "[Connection Worker@{:?}]: Unable save state in a State Store. {:?}",
          self.addr, err
        );
      }
    }

    self.disconnect.0.send(()).await.expect(&format!(
      "[Connection Worker@{:?}]: Unable to disconnect a client",
      self.addr
    ));

    send_control!(
      ControlMessage::ClientDisconnected {
        addr: self.addr.clone(),
        clean_session: self.state.has_clean_session(),
        client_id: client_id.clone(),
        will_packet: None,
      },
      self
    );
  }

  async fn shut_down(&mut self) {
    if let Ok(connected_state) = self.state.into_closed() {
      let client_id = connected_state.client_id.clone();
      if let Err(err) = self
        .state_store
        .write()
        .await
        .save_state(connected_state)
        .await
      {
        error!(
          "[Connection Worker@{:?}]: Unable save state in a State Store. {:?}",
          self.addr, err
        );
      }

      let disconnect_message = ControlMessage::ClientDisconnected {
        addr: self.addr.clone(),
        clean_session: self.state.has_clean_session(),
        client_id,
        will_packet: None,
      };
      send_control!(disconnect_message, self);
    }
  }

  async fn pingreq(&mut self, control_packet: ControlPacket) {
    send_stats!(
      StatsMessage::new_packet_processed(id!(self), &control_packet,),
      self
    );

    let pingres_packet = PingrespPacketBuilder::new().build();
    send_or_disconnect!(&pingres_packet, self);
  }

  async fn subscribe(&mut self, control_packet: ControlPacket) {
    send_stats!(
      StatsMessage::new_packet_processed(id!(self), &control_packet),
      self
    );

    let variable = match control_packet.variable {
      Variable::Subscribe(variable) => variable,
      _ => {
        error!(
          "[Connection Worker@{:?}]: Unexpected type of a variable",
          self.addr
        );
        disconnect!(self);
        return;
      }
    };
    let topic_subs = &variable.subscriptions;
    let packet_id = variable.packet_id;
    let subscriptions = topic_subs
      .iter()
      .map(|sub| sub.topic_filter.clone())
      .collect::<Vec<Subscription>>();
    let subscription_check = self.check_subscriptions(subscriptions.as_slice());

    let mut allowed_subscriptions: Vec<TopicSubscription> = Vec::with_capacity(subscriptions.len());

    for (i, allowed) in subscription_check.iter().enumerate() {
      if *allowed {
        allowed_subscriptions.push(topic_subs[i].clone());
      }
    }

    let subs_check: Vec<(bool, TopicSubscription)> = subscription_check
      .iter()
      .cloned()
      .zip(topic_subs.iter().cloned())
      .collect();

    if let Err(err) = self.state.subscribe(allowed_subscriptions.clone()) {
      error!(
        "[Connection Worker@{:?}]: Unable to add subscriptoins to a connection state. {:?}",
        self.addr, err
      );
    }

    let return_codes = subs_check
      .iter()
      .map(|(allowed, topic)| {
        if !allowed {
          return SubackReturnCode::Failure;
        }

        match topic.qos {
          QoS::Zero => SubackReturnCode::SuccessZero,
          QoS::One => SubackReturnCode::SuccessOne,
          QoS::Two => SubackReturnCode::SuccessTwo,
        }
      })
      .collect();

    let package = SubackPacketBuilder::new(packet_id)
      .with_return_codes(return_codes)
      .build();

    send_or_disconnect!(&package, self);

    send_control!(
      ControlMessage::AddSubscriptions {
        addr: self.addr.clone(),
        subscriptions: allowed_subscriptions
          .iter()
          .map(|s| s.topic_filter.clone())
          .collect(),
        client_id: id!(self)
      },
      self
    );
  }

  async fn unsubscribe(&mut self, control_packet: ControlPacket) {
    send_stats!(
      StatsMessage::new_packet_processed(id!(self), &control_packet),
      self
    );

    match control_packet.variable {
      Variable::Unsubscribe(UnsubscribeVariable {
        packet_id,
        subscriptions: to_unsubscribe,
      }) => {
        send_control!(
          ControlMessage::RemoveSubscriptions {
            addr: self.addr.clone(),
            subscriptions: to_unsubscribe.clone(),
            client_id: id!(self),
          },
          self
        );
        if let Err(err) = self.state.unsubscribe(to_unsubscribe) {
          error!(
            "[Connection Worker@{:?}]: Unable to unsubscribe. {:?}",
            self.addr, err
          );
          disconnect!(self);
          return;
        }
        let unsuback_packet = UnsubackPacketBuilder::new(packet_id).build();
        send_or_disconnect!(&unsuback_packet, self);
      }
      _ => {
        error!("[Connecton Worker]. Variable header does not match CPType");
        disconnect!(self);
      }
    }
  }

  async fn publish(&mut self, control_packet: ControlPacket) {
    send_stats!(
      StatsMessage::new_packet_processed(id!(self), &control_packet),
      self
    );

    let variable = match &control_packet.variable {
      &Variable::Publish(ref variable) => variable,
      _ => {
        error!(
          "[Connection Worker@{:?}]: Variable Header type does not match packet CPType",
          self.addr
        );
        disconnect!(self);
        return;
      }
    };
    let topic = &variable.topic_name;
    let allowed = self.check_publish(&topic);

    if !allowed {
      info!(
        "[Connection Worker@{:?}]: Unable to publish to {:?}. Publish is not allowed.",
        self.addr, topic
      );
      return;
    }

    send_control!(
      ControlMessage::Publish {
        addr: self.addr.clone(),
        packet: control_packet.clone(),
        client_id: id!(self)
      },
      self
    );

    let maybe_packet_id = getters_setters::get_packet_id(&control_packet.variable);
    match self
      .state
      .create_receive_transaction_from_packet(control_packet.clone())
    {
      Ok(qos) => {
        let packet_id = match maybe_packet_id {
          Some(p) => p,
          None => {
            return;
          }
        };
        // transaction created, qos > 0
        let confirmation_packet = match qos {
          QoS::One => PubackPacketBuilder::new(packet_id).build(),
          QoS::Two => PubrecPacketBuilder::new(packet_id).build(),
          QoS::Zero => return,
        };

        send_or_disconnect!(&confirmation_packet, self);

        match qos {
          QoS::One => {
            if let Err(err) = self.state.pubacked(&packet_id) {
              error!("Unable to puback packet. {:?}", err);
            }
          }
          QoS::Two => {
            if let Err(err) = self.state.pubreced(&packet_id) {
              error!("Unable to pubrel packet. {:?}", err);
            }
          }
          QoS::Zero => {
            // unreachable
          }
        }
      }
      Err(err) => {
        error!("Unable to create receive transaction. Error {:?}", err);
      }
    }
  }

  async fn forward_publish(&mut self, control_packet: ControlPacket, retained_for: Option<String>) {
    let (topic, qos) = match (
      &control_packet.variable,
      get_qos_level(&control_packet.fixed_header),
    ) {
      (Variable::Publish(ref variable), Ok(qos)) => (variable.topic_name.clone(), qos),
      _ => {
        error!("Malformed publish packet");
        return;
      }
    };

    match retained_for {
      Some(original_topic) => {
        if let Some(sub_qos) = self.state.get_topic_subscriptin_qos(original_topic) {
          self.send_once(&qos, &sub_qos, &control_packet).await;
        }
      }
      None => {
        for qos_iter in &self.state.get_subscription_qoss(&topic) {
          self.send_once(&qos, qos_iter, &control_packet).await;
        }
      }
    }
  }

  async fn send_once(&mut self, qos: &QoS, qos_iter: &QoS, control_packet: &ControlPacket) {
    let mut packet_to_send = control_packet.clone();
    let mut qos_to_use = qos;
    if qos > qos_iter {
      // QoS is bigger than a maximal QoS acceptable by a client
      info!("Downgrading QoS from {:?} to {:?}", qos, qos_iter);
      set_qos_level(&mut packet_to_send.fixed_header, qos_iter);
      qos_to_use = qos_iter;
    }

    let new_packet_id = if qos_to_use == &QoS::One || qos_to_use == &QoS::Two {
      match self
        .state
        .create_send_transaction_from_packet(&packet_to_send.clone())
      {
        Ok(packet_id) => packet_id,
        Err(err) => {
          error!("Unable to create a send transaction. Error {:?}", err);
          return;
        }
      }
    } else {
      None
    };

    match new_packet_id {
      Some(id) => {
        getters_setters::set_packet_id(&mut packet_to_send.variable, id);
      }
      None => {
        // QOS 0
        if let Err(err) = getters_setters::erase_packet_id(&mut packet_to_send.variable) {
          error!("{:?}", err);
        }
      }
    }

    send_or_disconnect!(&packet_to_send, self);
  }

  async fn puback(&mut self, control_packet: &ControlPacket) {
    send_stats!(
      StatsMessage::new_packet_processed(id!(self), &control_packet),
      self
    );

    let packet_id = match getters_setters::get_packet_id(&control_packet.variable) {
      Some(id) => id,
      None => {
        error!("Unable find packet ID in Puback packet.");
        return;
      }
    };

    if let Err(err) = self.state.puback(&packet_id) {
      error!("Unable to puback packet {:?}. Error {:?}", packet_id, err);
    }
  }

  async fn pubcomp(&mut self, control_packet: &ControlPacket) {
    send_stats!(
      StatsMessage::new_packet_processed(id!(self), &control_packet),
      self
    );

    let packet_id = match getters_setters::get_packet_id(&control_packet.variable) {
      Some(id) => id,
      None => {
        error!("Unable find packet ID in Pubcomp packet.");
        return;
      }
    };

    if let Err(err) = self.state.pubcomp(&packet_id) {
      error!("Unable to pubcomp packet {:?}. Error {:?}", packet_id, err);
    }
  }

  async fn pubrec(&mut self, control_packet: &ControlPacket) {
    send_stats!(
      StatsMessage::new_packet_processed(id!(self), &control_packet),
      self
    );

    let packet_id = match getters_setters::get_packet_id(&control_packet.variable) {
      Some(id) => id,
      None => {
        error!("Unable find packet ID in Pubrec packet.");
        return;
      }
    };

    if let Err(err) = self.state.pubrec(&packet_id) {
      error!("Unable to pubrec packet {:?}. Error {:?}", packet_id, err);
    }

    let pubrel_packet = PubrelPacketBuilder::new(packet_id).build();
    send_or_disconnect!(&pubrel_packet, self);
  }

  async fn pubrel(&mut self, control_packet: &ControlPacket) {
    send_stats!(
      StatsMessage::new_packet_processed(id!(self), &control_packet),
      self
    );

    let packet_id = match getters_setters::get_packet_id(&control_packet.variable) {
      Some(id) => id,
      None => {
        error!("Unable find packet ID in Pubrel packet.");
        return;
      }
    };

    if let Err(err) = self.state.pubrel(&packet_id) {
      error!("Unable to pubrel packet {:?}. Error {:?}", packet_id, err);
    }

    let pubcom_packet = PubcompPacketBuilder::new(packet_id).build();

    send_or_disconnect!(&pubcom_packet, self);

    if let Err(err) = self.state.pubcomped(&packet_id) {
      error!(
        "Unable to mark packet {:?} as pubcomp-ed. Error {:?}",
        packet_id, err
      );
    }
  }

  fn check_subscriptions(&self, subscriptions: &[Subscription]) -> Vec<bool> {
    match self.acl {
      Some(ref client_rules) => {
        let mut results: Vec<bool> = Vec::with_capacity(subscriptions.len());
        for sub in subscriptions {
          match client_rules
            .topics_acl
            .iter()
            .find(|r| topics_match(&sub.path, &r.topic.path))
          {
            Some(topic_rule) => match topic_rule.access {
              TopicAccess::ReadWrite | TopicAccess::Read => {
                results.push(true);
              }
              TopicAccess::Deny | TopicAccess::Write => {
                results.push(false);
              }
            },
            None => {
              results.push(false);
            }
          }
        }
        return results;
      }
      None => {
        return subscriptions.iter().map(|_| true).collect();
      }
    }
  }

  fn check_publish(&self, topic: &Topic) -> bool {
    match self.acl {
      Some(ref client_rules) => match client_rules
        .topics_acl
        .iter()
        .find(|r| topics_match(&topic.path, &r.topic.path))
      {
        Some(topic_rule) => match topic_rule.access {
          TopicAccess::ReadWrite | TopicAccess::Write => {
            return true;
          }
          TopicAccess::Deny | TopicAccess::Read => {
            return false;
          }
        },
        None => {
          return false;
        }
      },
      None => {
        return true;
      }
    }
  }
}
