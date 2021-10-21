use bytes::BytesMut;

use super::topic_subscription::TopicSubscription;
use crate::v_3_1_1::PacketId;

#[derive(Debug, PartialEq, Clone)]
pub struct Variable {
  /// The Packet Identifier field.
  pub packet_id: PacketId,
  pub subscriptions: Vec<TopicSubscription>,
}

impl Variable {
  pub const PACKET_ID_LEN: usize = 2;
}

pub struct VariableCodec;

impl VariableCodec {
  pub const PACKET_ID_LEN: usize = Variable::PACKET_ID_LEN;

  /// Factory method that creates new instance of `VariableHeaderCodec`.
  pub fn new() -> Self {
    VariableCodec {}
  }

  pub fn encode(&mut self, item: &Variable, dst: &mut BytesMut) -> Result<(), std::io::Error> {
    dst.extend_from_slice(item.packet_id.as_slice());
    for topic in &item.subscriptions {
      topic.encode(dst)?;
    }

    Ok(())
  }

  pub fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Variable>, std::io::Error> {
    let packet_id = src.split_to(Self::PACKET_ID_LEN).to_vec();
    let mut subscriptions = vec![];
    while src.len() > 0 {
      subscriptions.push(TopicSubscription::decode(src)?);
    }

    Ok(Some(Variable {
      packet_id,
      subscriptions,
    }))
  }
}
