use bytes::{BufMut, BytesMut};

use crate::v_3_1_1::topic::Subscription;
use crate::v_3_1_1::utils::codec as utils_codec;
use crate::v_3_1_1::QoS;

#[derive(Debug, PartialEq, Clone)]
pub struct TopicSubscription {
    /// Subscription topic filter
    pub topic_filter: Subscription,

    /// The maximum QoS level at which
    /// the Server can send Application Messages to the Client
    pub qos: QoS,
}

impl TopicSubscription {
    const RESERVED_BYTES_MASK: u8 = 0b11111100;
    const EXPECTED_RESERVED_BYTES: u8 = 0b00000000;

    pub fn new(topic_filter: Subscription, qos: QoS) -> Self {
        TopicSubscription { topic_filter, qos }
    }

    pub fn encode(&self, dst: &mut BytesMut) -> std::io::Result<()> {
        utils_codec::encode_string(&self.topic_filter.original, dst);
        dst.put_u8(self.qos.bits());

        Ok(())
    }

    pub fn decode(src: &mut BytesMut) -> std::io::Result<Self> {
        let topic_filter = utils_codec::decode_optional_string(src)
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Subscription payload codec: empty topic filter",
            ))
            .and_then(Subscription::try_from)?;

        let qos_byte = src.split_to(1)[0];

        if qos_byte & Self::RESERVED_BYTES_MASK != Self::EXPECTED_RESERVED_BYTES {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Subscription payload codec: requested QoS reserved bits contain non-zero values",
            ));
        }

        let qos = QoS::try_from(qos_byte)?;

        Ok(TopicSubscription { topic_filter, qos })
    }
}
