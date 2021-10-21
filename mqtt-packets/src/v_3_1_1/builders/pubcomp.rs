use crate::v_3_1_1::basic_variable::Variable as BasicVariable;
use crate::v_3_1_1::cp_fixed_header::FixedHeader;
use crate::v_3_1_1::variable::Variable;
use crate::v_3_1_1::PacketId;
use crate::v_3_1_1::{CPRemLen, CPType, ControlPacket, Flag};

pub struct PubcompPacketBuilder {
    packet: ControlPacket,
}

impl PubcompPacketBuilder {
    pub fn new(packet_id: &PacketId) -> Self {
        PubcompPacketBuilder {
            packet: ControlPacket {
                fixed_header: FixedHeader {
                    cp_type: CPType::Pubcomp,
                    flag: Flag {
                        control_packet: CPType::Pubcomp,
                        is_reserved: true,
                        bits: 0,
                    },
                    remaining_length: CPRemLen::new(2),
                },
                variable: Variable::Pubcomp(BasicVariable {
                    packet_id: packet_id.clone(),
                }),
            },
        }
    }

    pub fn build(self) -> ControlPacket {
        self.packet
    }
}
