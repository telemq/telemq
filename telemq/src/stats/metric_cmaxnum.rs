use super::{message::StatsMessage, metric_trait::StatsMetric};
use std::collections::HashSet;

type ClientId = String;

pub struct MetricCMaxNum {
  clients: HashSet<ClientId>,
}

impl MetricCMaxNum {
  pub fn new() -> MetricCMaxNum {
    MetricCMaxNum {
      clients: HashSet::new(),
    }
  }
}

impl StatsMetric for MetricCMaxNum {
  // TODO: declare it as a part of a trait common for all metrics
  fn get_value(&self) -> Vec<u8> {
    self.clients.len().to_be_bytes().to_vec()
  }

  fn update(&mut self, message: &StatsMessage) {
    match message {
      StatsMessage::ClientConnected { client_id, .. } => {
        self.clients.insert(client_id.clone());
      }
      StatsMessage::ClientDisconnected { client_id, .. } => {
        self.clients.remove(client_id);
      }
      _ => {}
    }
  }
}
