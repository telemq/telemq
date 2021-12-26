use super::message::StatsMessage;
use log::error;
use std::collections::{HashMap, HashSet};

/// Statistics state difference item, represented as a tuple
/// `(path, new_value)`. Where `path` is a string of `a/b/c` (for example, `clients/current`) form
/// which can be eventually transformed to a $SYS topic via `format!("$SYS/{}", path)`,
/// `new_value` is a binary representation of a new value that can be used
/// as a payload for a Publish control packet with a $SYS topic.
pub type StatsStateDiffItem = (String, Vec<u8>);

pub struct StatsState {
  current: StatsStateInner,
  prev: StatsStateInner,
  is_pristine: bool,
}

impl StatsState {
  pub fn new() -> StatsState {
    StatsState {
      is_pristine: true,
      current: StatsStateInner::new(),
      prev: StatsStateInner::new(),
    }
  }

  pub fn update(&mut self, message: StatsMessage) {
    self.is_pristine = false;

    self.current.update(message);
  }

  /// Compares a current inner state with a previos checkpoint,
  /// returns a difference and creates a new checkpoint.
  pub fn checkpoint(&mut self) -> Vec<StatsStateDiffItem> {
    if self.is_pristine {
      return vec![];
    }

    let diff = self.prev.compare_with(&self.current);
    self.prev = self.current.clone();
    self.is_pristine = true;

    diff
  }
}

#[derive(Clone)]
struct StatsStateInner {
  clients_online: HashSet<String>,
  metrics: HashMap<&'static str, u128>,
}

impl StatsStateInner {
  const BROKER_BYTES_RECEIVED_NAME: &'static str = "broker/bytes/received";
  const BROKER_BYTES_SENT_NAME: &'static str = "broker/bytes/received";
  const BROKER_MESSAGES_RECEIVED_NAME: &'static str = "broker/messages/received";
  const BROKER_MESSAGES_SENT_NAME: &'static str = "broker/messages/received";
  const BROKER_CLIENTS_CONNECTED: &'static str = "broker/clients/connected";
  const BROKER_CLIENTS_MAXIMUM: &'static str = "broker/clients/maximum";

  fn new() -> Self {
    let mut metrics = HashMap::new();
    metrics.insert(Self::BROKER_BYTES_RECEIVED_NAME, 0u8.into());
    metrics.insert(Self::BROKER_BYTES_SENT_NAME, 0u8.into());
    metrics.insert(Self::BROKER_MESSAGES_RECEIVED_NAME, 0u8.into());
    metrics.insert(Self::BROKER_MESSAGES_SENT_NAME, 0u8.into());
    metrics.insert(Self::BROKER_CLIENTS_CONNECTED, 0u8.into());
    metrics.insert(Self::BROKER_CLIENTS_MAXIMUM, 0u8.into());
    let clients_online = HashSet::new();

    StatsStateInner {
      metrics,
      clients_online,
    }
  }

  fn update(&mut self, message: StatsMessage) {
    match message {
      StatsMessage::ClientConnected { client_id, .. } => {
        self.on_client_connected(client_id);
      }
      StatsMessage::ClientDisconnected { client_id, .. } => {
        self.on_client_disconnected(client_id);
      }
      StatsMessage::PacketProcessedReceived { bytes, .. } => {
        self.on_packet_processed_received(bytes);
      }
      StatsMessage::PacketProcessedSend { bytes, .. } => {
        self.on_packet_processed_sent(bytes);
      }
    }
  }

  fn compare_with(&self, other_state: &StatsStateInner) -> Vec<StatsStateDiffItem> {
    let mut diff = vec![];

    for (k, v) in &other_state.metrics {
      match self.metrics.get(k) {
        Some(self_value) => {
          if v != self_value {
            diff.push((k.to_string(), v.to_be_bytes().to_vec()));
          }
        }
        None => {
          error!("BUG: Two StatsStatesInner should have the same metrics registered. \"{}\" not found in previous checkpoint", k);
          diff.push((k.to_string(), v.to_be_bytes().to_vec()));
        }
      }
    }

    diff
  }

  fn on_client_connected(&mut self, client_id: String) {
    self.clients_online.insert(client_id);
    let currently_clients = self.clients_online.len() as u128;

    if let Some(v) = self.metrics.get_mut(Self::BROKER_CLIENTS_CONNECTED) {
      *v = currently_clients;
    }

    if let Some(v) = self.metrics.get_mut(Self::BROKER_CLIENTS_MAXIMUM) {
      if *v < currently_clients {
        *v = currently_clients;
      }
    }
  }

  fn on_client_disconnected(&mut self, client_id: String) {
    self.clients_online.remove(&client_id);
    let currently_clients = self.clients_online.len() as u128;

    if let Some(v) = self.metrics.get_mut(Self::BROKER_CLIENTS_CONNECTED) {
      *v = currently_clients;
    }

    if let Some(v) = self.metrics.get_mut(Self::BROKER_CLIENTS_MAXIMUM) {
      if *v < currently_clients {
        *v = currently_clients;
      }
    }
  }

  fn on_packet_processed_received(&mut self, bytes: u64) {
    if let Some(v) = self.metrics.get_mut(Self::BROKER_BYTES_RECEIVED_NAME) {
      *v += bytes as u128;
    }
    if let Some(v) = self.metrics.get_mut(Self::BROKER_MESSAGES_RECEIVED_NAME) {
      *v += 1u128;
    }
  }

  fn on_packet_processed_sent(&mut self, bytes: u64) {
    if let Some(v) = self.metrics.get_mut(Self::BROKER_BYTES_SENT_NAME) {
      *v += bytes as u128;
    }
    if let Some(v) = self.metrics.get_mut(Self::BROKER_MESSAGES_SENT_NAME) {
      *v += 1u128;
    }
  }
}
