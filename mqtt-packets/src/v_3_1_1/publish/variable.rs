use bytes::BytesMut;

use crate::v_3_1_1::topic::Topic;
use crate::v_3_1_1::utils::codec as codec_utils;
use crate::v_3_1_1::QoS;

use super::super::packet_id::PacketId;

/// Publish packet variable header
#[derive(Debug, PartialEq, Clone)]
pub struct Variable {
    //
    // Variable header
    //
    /// The Topic Name identifies the information channel to which payload data is published.
    pub topic_name: Topic,

    /// The Packet Identifier field is only present in PUBLISH Packets where the QoS level is 1 or 2.
    pub packet_id: Option<PacketId>,

    pub payload: Vec<u8>,
}

pub struct VariableCodec {
    qos: QoS,
}

impl VariableCodec {
    const PACKET_ID_LEN: usize = 2;

    pub fn new(qos: QoS) -> Self {
        VariableCodec { qos }
    }

    pub fn encode(&mut self, item: &Variable, dst: &mut BytesMut) -> Result<(), std::io::Error> {
        codec_utils::encode_optional_string(&Some(&item.topic_name.original), dst);
        if let Some(ref packet_id) = item.packet_id {
            dst.extend_from_slice(packet_id.as_slice());
        }
        dst.extend_from_slice(item.payload.as_slice());
        Ok(())
    }

    pub fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Variable>, std::io::Error> {
        // TODO: refactor downstream codecs to avoid unwrapping
        let topic_name = Topic::try_from(codec_utils::decode_optional_string(src).unwrap())?;
        let should_have_packet_id = self.qos == QoS::One || self.qos == QoS::Two;
        let packet_id = if should_have_packet_id {
            Some(src.split_to(Self::PACKET_ID_LEN).to_vec())
        } else {
            None
        };
        let payload = src.to_vec();

        Ok(Some(Variable {
            packet_id,
            topic_name,
            payload,
        }))
    }
}
