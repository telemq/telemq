use bytes::{BufMut, BytesMut};

use crate::v_3_1_1::topic::Subscription;
use crate::v_3_1_1::utils::codec as utils_codec;
use crate::v_3_1_1::PacketId;

#[derive(Debug, PartialEq, Clone)]
pub struct Variable {
    /// The Packet Identifier field.
    pub packet_id: PacketId,
    pub subscriptions: Vec<Subscription>,
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
        for topic_filter in &item.subscriptions {
            let encoded = topic_filter.original.as_bytes();
            dst.put_u16(encoded.len() as u16);
            dst.extend_from_slice(encoded);
        }

        Ok(())
    }

    pub fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Variable>, std::io::Error> {
        let packet_id = src.split_to(Self::PACKET_ID_LEN).to_vec();
        let mut subscriptions = vec![];

        while src.len() > 0 {
            match utils_codec::decode_optional_string(src) {
                Some(topic) => subscriptions.push(Subscription::try_from(topic)?),
                None => break,
            }
        }

        Ok(Some(Variable {
            packet_id,
            subscriptions,
        }))
    }
}
