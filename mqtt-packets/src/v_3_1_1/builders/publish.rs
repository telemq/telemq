use crate::v_3_1_1::cp_fixed_header::FixedHeader;
use crate::v_3_1_1::publish::fixed_header::{set_dup, set_qos_level, set_retained};
use crate::v_3_1_1::publish::variable::Variable as PublishVariable;
use crate::v_3_1_1::topic::Topic;
use crate::v_3_1_1::variable::Variable;
use crate::v_3_1_1::PacketId;
use crate::v_3_1_1::{CPRemLen, CPType, ControlPacket, Flag, QoS};

pub struct PublishPacketBuilder {
    packet: ControlPacket,
}

impl PublishPacketBuilder {
    pub fn new() -> Self {
        PublishPacketBuilder {
            packet: ControlPacket {
                fixed_header: FixedHeader {
                    cp_type: CPType::Publish,
                    flag: Flag {
                        control_packet: CPType::Publish,
                        is_reserved: false,
                        bits: 0,
                    },
                    remaining_length: CPRemLen::new(0),
                },
                variable: Variable::Publish(PublishVariable {
                    packet_id: None,
                    topic_name: Topic::try_from("EMPTY").unwrap(),
                    payload: vec![],
                }),
            },
        }
    }

    pub fn with_dup(&mut self, is_dup: bool) -> &mut Self {
        set_dup(&mut self.packet.fixed_header, is_dup);

        self
    }

    pub fn with_retained(&mut self, is_retained: bool) -> &mut Self {
        set_retained(&mut self.packet.fixed_header, is_retained);

        self
    }

    pub fn with_qos(&mut self, qos: &QoS) -> &mut Self {
        set_qos_level(&mut self.packet.fixed_header, qos);
        self
    }

    pub fn with_packet_id(&mut self, packet_id: PacketId) -> &mut Self {
        if let Variable::Publish(ref mut variable) = self.packet.variable {
            variable.packet_id = Some(packet_id);
        } else {
            unreachable!();
        }

        self
    }

    pub fn with_topic(&mut self, topic_name: Topic) -> &mut Self {
        if let Variable::Publish(ref mut variable) = self.packet.variable {
            variable.topic_name = topic_name;
        } else {
            unreachable!();
        }

        self
    }

    pub fn with_payload(&mut self, payload: Vec<u8>) -> &mut Self {
        if let Variable::Publish(ref mut variable) = self.packet.variable {
            variable.payload = payload;
        } else {
            unreachable!();
        }

        self
    }

    pub fn build(self) -> ControlPacket {
        self.packet
    }

    pub fn produce(&self) -> ControlPacket {
        self.packet.clone()
    }
}
