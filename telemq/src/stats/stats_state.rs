use super::message::StatsMessage;
use std::collections::{HashMap, HashSet};

/// Statistics state difference item, represented as a tuple
/// `(path, new_value)`. Where `path` is a string of `a/b/c` (for example, `clients/current`) form
/// which can be eventually transformed to a $SYS topic via `format!("$SYS/{}", path)`,
/// `new_value` is a binary representation of a new value that can be used
/// as a payload for a Publish control packet with a $SYS topic.
pub type StatsStateView = (String, String);

pub struct StatsState {
    current: StatsStateInner,
}

impl StatsState {
    pub fn new() -> StatsState {
        StatsState {
            current: StatsStateInner::new(),
        }
    }

    pub fn update(&mut self, message: StatsMessage) {
        self.current.update(message);
    }

    /// Returns a list of metrics views.
    pub fn checkpoint(&mut self) -> Vec<StatsStateView> {
        self.current.get_metrics()
    }
}

#[derive(Clone)]
struct StatsStateInner {
    clients_online: HashSet<String>,
    metrics: HashMap<&'static str, u128>,
}

impl StatsStateInner {
    const BROKER_BYTES_RECEIVED_NAME: &'static str = "broker/bytes/received";
    const BROKER_BYTES_SENT_NAME: &'static str = "broker/bytes/sent";
    const BROKER_MESSAGES_RECEIVED_NAME: &'static str = "broker/messages/received";
    const BROKER_MESSAGES_SENT_NAME: &'static str = "broker/messages/sent";
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

    fn get_metrics(&self) -> Vec<StatsStateView> {
        let mut metrics = vec![];

        for (k, v) in &self.metrics {
            metrics.push((k.to_string(), format!("{}", v)));
        }

        metrics
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
