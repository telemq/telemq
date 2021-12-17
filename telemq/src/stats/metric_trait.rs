use super::message::StatsMessage;

pub trait StatsMetric: Send + Sync {
  // TODO: declare it as a part of a trait common for all metrics
  fn get_value(&self) -> Vec<u8>;

  fn update(&mut self, message: &StatsMessage);
}
