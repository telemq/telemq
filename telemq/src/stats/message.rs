use mqtt_packets::v_3_1_1::ControlPacket;
use std::net::SocketAddr;

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
