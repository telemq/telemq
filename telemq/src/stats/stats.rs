use super::{
    message::StatsMessage,
    stats_state::{StatsState, StatsStateView},
};
use crate::control::{ControlMessage, ControlSender};
use log::{error, info};
use mqtt_packets::v_3_1_1::{builders::PublishPacketBuilder, topic::Topic, ControlPacket};
use std::{io, time::Duration};
use tokio::{
    select,
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    time::interval,
};

pub type StatsSender = UnboundedSender<StatsMessage>;
pub type StatsReceiver = UnboundedReceiver<StatsMessage>;

pub struct StatsConfig {
    pub update_interval: Duration,
    pub control_sender: ControlSender,
}

pub struct Stats {
    receiver: StatsReceiver,
    state: StatsState,
    update_interval: Duration,
    control_sender: ControlSender,
}

impl Stats {
    pub fn new(config: StatsConfig) -> (Self, StatsSender) {
        let (sender, receiver) = unbounded_channel();

        (
            Stats {
                receiver,
                state: StatsState::new(),
                update_interval: config.update_interval,
                control_sender: config.control_sender,
            },
            sender,
        )
    }

    pub async fn run(mut self) -> io::Result<()> {
        if self.update_interval.is_zero() {
            info!("[Stats Worker]: update interval is zero. Ingore incomming messages");
            loop {
                // do nothing with a message
                self.receiver.recv().await;
            }
        } else {
            let mut interval_stream = interval(self.update_interval);
            loop {
                select! {
                  Some(stats_message) = self.receiver.recv() => {
                    self.state.update(stats_message);
                  },
                  _ = interval_stream.tick() => {
                    let metrics = self.state.checkpoint();
                    for mtr in metrics {
                      let packet = Self::build_publish_packet(mtr);
                      if let Err(err) = self.control_sender.send(ControlMessage::Publish{
                        addr: None,
                        client_id: None,
                        packet
                      }) {
                        error!("[Stats Worker]: Unable to publish stats update - {:?}", err);
                      }
                    }
                  }
                }
            }
        }
    }

    fn build_publish_packet(d: StatsStateView) -> ControlPacket {
        let sys_topic = Topic::make_from_string(format!("$SYS/{}", d.0));
        let mut builder = PublishPacketBuilder::new();
        builder
            .with_topic(sys_topic)
            .with_payload(d.1.as_bytes().to_vec());

        builder.build()
    }
}
