use super::{
  message::StatsMessage, metric_cmaxnum::MetricCMaxNum, metric_cnum::MetricCNum,
  metric_trait::StatsMetric,
};
use std::{collections::HashMap, io};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

pub type StatsSender = UnboundedSender<StatsMessage>;
pub type StatsReceiver = UnboundedReceiver<StatsMessage>;

pub struct Stats {
  receiver: StatsReceiver,
  metrics: HashMap<String, Box<dyn StatsMetric>>,
}

impl Stats {
  pub fn new() -> (Self, StatsSender) {
    let (sender, receiver) = unbounded_channel();
    let mut metrics = HashMap::new();
    Self::register_metric(
      &mut metrics,
      "clients/connected".into(),
      Box::new(MetricCNum::new()),
    );
    Self::register_metric(
      &mut metrics,
      "clients/maximum".into(),
      Box::new(MetricCMaxNum::new()),
    );
    (Stats { receiver, metrics }, sender)
  }

  fn register_metric<M>(
    metrics: &mut HashMap<String, Box<dyn StatsMetric>>,
    key: String,
    new_metric: Box<M>,
  ) where
    M: StatsMetric + 'static + Send + Sync,
  {
    metrics.insert(key, new_metric);
  }

  pub async fn run(mut self) -> io::Result<()> {
    loop {
      if let Some(stats_message) = self.receiver.recv().await {
        for metric in self.metrics.values_mut() {
          metric.update(&stats_message);
        }
      }
    }
  }
}
