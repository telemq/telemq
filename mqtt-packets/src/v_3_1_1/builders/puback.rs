use crate::v_3_1_1::basic_variable::Variable as BasicVariable;
use crate::v_3_1_1::cp_fixed_header::FixedHeader;
use crate::v_3_1_1::variable::Variable;
use crate::v_3_1_1::PacketId;
use crate::v_3_1_1::{CPRemLen, CPType, ControlPacket, Flag};

pub struct PubackPacketBuilder {
    packet: ControlPacket,
}

impl PubackPacketBuilder {
    pub fn new(packet_id: &PacketId) -> Self {
        PubackPacketBuilder {
            packet: ControlPacket {
                fixed_header: FixedHeader {
                    cp_type: CPType::Puback,
                    flag: Flag {
                        control_packet: CPType::Puback,
                        is_reserved: true,
                        bits: 0,
                    },
                    remaining_length: CPRemLen::new(2),
                },
                variable: Variable::Puback(BasicVariable {
                    packet_id: packet_id.clone(),
                }),
            },
        }
    }

    pub fn build(self) -> ControlPacket {
        self.packet
    }
}
