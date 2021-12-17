use super::{message::StatsMessage, metric_trait::StatsMetric};
use std::collections::HashSet;

type ClientId = String;

pub struct MetricCNum {
  clients_online: HashSet<ClientId>,
}

impl MetricCNum {
  pub fn new() -> MetricCNum {
    MetricCNum {
      clients_online: HashSet::new(),
    }
  }
}

impl StatsMetric for MetricCNum {
  // TODO: declare it as a part of a trait common for all metrics
  fn get_value(&self) -> Vec<u8> {
    self.clients_online.len().to_be_bytes().to_vec()
  }

  fn update(&mut self, message: &StatsMessage) {
    match message {
      StatsMessage::ClientConnected { client_id, .. } => {
        self.clients_online.insert(client_id.clone());
      }
      StatsMessage::ClientDisconnected { client_id, .. } => {
        self.clients_online.remove(client_id);
      }
      _ => {}
    }
  }
}
