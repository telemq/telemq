use mqtt_packets::v_3_1_1::ControlPacket;
use std::{io, net::SocketAddr};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

pub type StatsSender = UnboundedSender<StatsMessage>;
pub type StatsReceiver = UnboundedReceiver<StatsMessage>;

#[derive(Debug)]
pub enum StatsMessage {
  ClientConnected { client_id: String, addr: SocketAddr },
  ClientDisconnected { client_id: String },
  PacketProcessed { client_id: String, bytes: u64 },
}

impl StatsMessage {
  pub fn new_packet_processed(client_id: String, control_packet: &ControlPacket) -> StatsMessage {
    let bytes = (control_packet.fixed_header.remaining_length.as_value() + 1) as u64;

    StatsMessage::PacketProcessed { client_id, bytes }
  }

  pub fn get_name(&self) -> String {
    match self {
      Self::ClientConnected { .. } => "StatsMessage::ClientConnected".into(),
      Self::ClientDisconnected { .. } => "StatsMessage::ClientDisconnected".into(),
      Self::PacketProcessed { .. } => "StatsMessage::PacketProcessed".into(),
    }
  }
}

pub struct Stats {
  receiver: StatsReceiver,
}

impl Stats {
  pub fn new() -> (Self, StatsSender) {
    let (sender, receiver) = unbounded_channel();
    (Stats { receiver }, sender)
  }

  pub async fn run(mut self) -> io::Result<()> {
    loop {
      if let Some(stats_message) = self.receiver.recv().await {
        match stats_message {
          StatsMessage::ClientConnected { client_id, addr } => {
            println!("Client Connected {:?} {:?}\n\n", client_id, addr);
          }
          StatsMessage::ClientDisconnected { client_id } => {
            println!("Client Disconnected {:?}\n\n", client_id);
          }
          StatsMessage::PacketProcessed { client_id, bytes } => {
            // println!("Packet Processed {:?} {:?}\n\n", client_id, bytes);
          }
        }
      }
    }
  }
}
