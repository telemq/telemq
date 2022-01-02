use crate::{
  config::TeleMQServerConfig,
  connection::{ConnectionMessage, ConnectionSender},
  session_state_store::SessionStateStore,
  subscription_tree::SubscriptionTree,
};
use futures::future::join_all;
use log::{error, info};
use mqtt_packets::v_3_1_1::{
  publish::fixed_header::is_retained,
  topic::{Subscription, Topic},
  variable::Variable,
  ControlPacket,
};
use std::{collections::HashMap, io, net::SocketAddr, sync::Arc};
use tokio::{
  select,
  sync::{
    mpsc::{unbounded_channel, Sender, UnboundedReceiver, UnboundedSender},
    RwLock,
  },
};

#[derive(Debug)]
pub enum ControlMessage {
  ClientConnected {
    addr: SocketAddr,
    client_id: String,
    clean_session: bool,
    sender: ConnectionSender,
  },
  ClientDisconnected {
    addr: SocketAddr,
    client_id: String,
    clean_session: bool,
    will_packet: Option<ControlPacket>,
  },
  AddSubscriptions {
    addr: SocketAddr,
    client_id: String,
    subscriptions: Vec<Subscription>,
  },
  RemoveSubscriptions {
    addr: SocketAddr,
    client_id: String,
    subscriptions: Vec<Subscription>,
  },
  Publish {
    addr: Option<SocketAddr>,
    client_id: Option<String>,
    packet: ControlPacket,
  },
  ShutDown,
}

impl ControlMessage {
  pub fn get_name(&self) -> String {
    match self {
      ControlMessage::ClientConnected { .. } => "ControlMessage::ClientConnected".into(),
      ControlMessage::ClientDisconnected { .. } => "ControlMessage::ClientDisconnected".into(),
      ControlMessage::AddSubscriptions { .. } => "ControlMessage::AddSubscriptions".into(),
      ControlMessage::RemoveSubscriptions { .. } => "ControlMessage::AddSubscriptions".into(),
      ControlMessage::Publish { .. } => "ControlMessage::Publish".into(),
      ControlMessage::ShutDown => "ControlMessage::ShutDown".into(),
    }
  }
}

pub type ControlSender = UnboundedSender<ControlMessage>;
pub type ControlReceiver = UnboundedReceiver<ControlMessage>;

type ClientId = String;

// TODO: add retained messages max lifetime to remove old ones
#[derive(Debug)]
pub struct Control {
  receiver: ControlReceiver,
  connections: HashMap<ClientId, ConnectionSender>,
  subscription_tree: SubscriptionTree,
  retained_messages: Vec<(Topic, ControlPacket)>,
  state_store: Arc<RwLock<SessionStateStore>>,
  is_shutting_down: bool,
  shut_down_channel: Sender<()>,
}

impl Control {
  pub async fn new(
    config: &TeleMQServerConfig,
    state_store: Arc<RwLock<SessionStateStore>>,
    shut_down_channel: Sender<()>,
  ) -> (Self, ControlSender) {
    let (tx, rx) = unbounded_channel();
    (
      Control {
        receiver: rx,
        connections: HashMap::with_capacity(config.max_connections),
        subscription_tree: SubscriptionTree::from_session_state_store(state_store.clone()).await,
        retained_messages: vec![],
        state_store,
        is_shutting_down: false,
        shut_down_channel,
      },
      tx,
    )
  }

  pub async fn run(mut self) -> io::Result<()> {
    loop {
      select! {
        Some(control_message) = self.receiver.recv() => {
          match control_message {
            ControlMessage::ClientConnected{sender, client_id, clean_session, ..} => {
              self.on_add_connection(sender, client_id, clean_session);
            },
            ControlMessage::AddSubscriptions{subscriptions, client_id, .. } => {
              self.on_add_subscriptions(client_id, subscriptions).await;
            }
            ControlMessage::RemoveSubscriptions{subscriptions, client_id, ..} => {
              self.on_remove_subscriptions(client_id, subscriptions);
            }
            ControlMessage::Publish{packet, ..} => {
              self.on_publish(packet).await;
            }
            ControlMessage::ClientDisconnected{client_id, clean_session, will_packet, ..} => {
              self.on_client_disconnect(client_id, clean_session, will_packet).await;
            }
            ControlMessage::ShutDown => {
              self.on_shut_down().await;
            }
          }
        }
      }
    }
  }

  fn on_add_connection(
    &mut self,
    sender: ConnectionSender,
    client_id: String,
    clean_session: bool,
  ) {
    if clean_session {
      self.subscription_tree.disconnect_subscriber(&client_id);
    }

    if let Some(connected_client_sender) = self.connections.remove(&client_id) {
      // there is already a connected client with the same id
      // disconnect it
      info!("Disconnecting already connected client {:?}", client_id);
      let message = ConnectionMessage::Disconnect;
      let message_type = message.get_name();
      if let Err(err) = connected_client_sender.send(message) {
        error!(
          "[Control Worker]: Unable to send {} to {:?}. {:?}",
          message_type, client_id, err
        );
      }
    }
    self.connections.insert(client_id, sender);
  }

  async fn on_add_subscriptions(&mut self, client_id: ClientId, subscriptions: Vec<Subscription>) {
    if self.connections.get(&client_id).is_none() {
      return;
    }

    for sub in &subscriptions {
      self
        .subscription_tree
        .add_subscriber(&sub.path, client_id.clone());
    }

    let mut futs = Vec::new();
    for sub in &subscriptions {
      for (topic, publish_packet) in &self.retained_messages {
        if sub.topic_matches(&topic) {
          futs.push(self.inform_connection(
            client_id.clone(),
            ConnectionMessage::Publish {
              packet: publish_packet.clone(),
              retained_for: Some(sub.original.clone()),
            },
          ));
        }
      }
    }

    join_all(futs).await;
  }

  fn on_remove_subscriptions(&mut self, client_id: ClientId, subscriptions: Vec<Subscription>) {
    for sub in subscriptions {
      self
        .subscription_tree
        .remove_subscriber(&sub.path, client_id.clone());
    }
  }

  async fn on_client_disconnect(
    &mut self,
    client_id: ClientId,
    clean_session: bool,
    will_packet: Option<ControlPacket>,
  ) {
    if let Some(to_send) = will_packet {
      self.on_publish(to_send).await;
    }

    if clean_session {
      self.subscription_tree.disconnect_subscriber(&client_id);
    }
    self.connections.remove(&client_id);

    if self.connections.is_empty() && self.is_shutting_down {
      self.shut_down_channel.send(()).await.unwrap();
    }
  }

  async fn on_publish(&mut self, control_packet: ControlPacket) {
    let variable = match &control_packet.variable {
      &Variable::Publish(ref variable) => variable,
      _ => {
        unreachable!();
      }
    };
    let topic = &variable.topic_name;

    if is_retained(&control_packet.fixed_header) {
      self
        .retained_messages
        .push((topic.clone(), control_packet.clone()));
    }

    let subscribers = self.subscription_tree.find_subscribers(&topic.path);
    println!(
      "SUB TREE {:?}.\nSubs {:?}",
      self.subscription_tree, subscribers
    );

    // allowed
    let mut futs = Vec::with_capacity(subscribers.len());
    for client_id in subscribers {
      futs.push(self.inform_connection(
        client_id.clone(),
        ConnectionMessage::Publish {
          packet: control_packet.clone(),
          retained_for: None,
        },
      ));
    }

    join_all(futs).await;
  }

  async fn on_shut_down(&mut self) {
    if let Err(err) = self.state_store.read().await.commit().await {
      error!("[Control Worker]: unable to commit State Store. {:?}", err);
    }

    if self.connections.is_empty() {
      self.shut_down_channel.send(()).await.unwrap();
      return;
    }

    self.is_shutting_down = true;

    for (con, ch) in &self.connections {
      if let Err(err) = ch.send(ConnectionMessage::ShutDown) {
        error!(
          "[Control Worker]: unable to gracefully shut down connection {:?}. {:?}",
          con, err
        );
      }
    }
  }

  async fn inform_connection(&self, client_id: ClientId, message: ConnectionMessage) {
    match self.connections.get(&client_id) {
      Some(connection_sender) => {
        let message_type = message.get_name();
        if let Err(err) = connection_sender.send(message) {
          error!(
            "[Control Worker]: Unable to send {} to {:?}. {:?}",
            message_type, client_id, err
          );
        }
      }
      None => {
        // so far there is only on ConnectionMessage type - Publish
        // for this case if client wasn't found among active connections
        // we assume it has a stored session (clean session = false) in
        // the Session State Store
        match message {
          ConnectionMessage::Publish { packet, .. } => {
            if let Err(err) = self
              .state_store
              .read()
              .await
              .new_publish(&client_id, packet)
              .await
            {
              error!(
                "[Control Worker]: Unable to update State Store with a new Publish. {:?}",
                err
              );
            }
          }
          _ => {}
        }
      }
    }
  }
}
