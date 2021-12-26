use mqtt_packets::v_3_1_1::ControlPacket;
use std::net::SocketAddr;

#[derive(Debug)]
pub enum StatsMessage {
  ClientConnected {
    client_id: String,
    clean_session: bool,
    addr: SocketAddr,
  },
  ClientDisconnected {
    client_id: String,
  },
  PacketProcessedSend {
    client_id: String,
    bytes: u64,
  },
  PacketProcessedReceived {
    client_id: String,
    bytes: u64,
  },
}

impl StatsMessage {
  pub fn new_packet_processed_received(
    client_id: String,
    control_packet: &ControlPacket,
  ) -> StatsMessage {
    let bytes = Self::bytes_number(control_packet);

    StatsMessage::PacketProcessedReceived { client_id, bytes }
  }

  pub fn new_packet_processed_send(
    client_id: String,
    control_packet: &ControlPacket,
  ) -> StatsMessage {
    let bytes = Self::bytes_number(control_packet);

    StatsMessage::PacketProcessedSend { client_id, bytes }
  }

  fn bytes_number(control_packet: &ControlPacket) -> u64 {
    (control_packet.fixed_header.remaining_length.as_value() + 1) as u64
  }

  pub fn get_name(&self) -> String {
    match self {
      Self::ClientConnected { .. } => "StatsMessage::ClientConnected".into(),
      Self::ClientDisconnected { .. } => "StatsMessage::ClientDisconnected".into(),
      Self::PacketProcessedReceived { .. } => "StatsMessage::PacketProcessedReceived".into(),
      Self::PacketProcessedSend { .. } => "StatsMessage::PacketProcessedReceived".into(),
    }
  }
}
