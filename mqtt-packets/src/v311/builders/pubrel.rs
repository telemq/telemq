use crate::v311::basic_variable::Variable as BasicVariable;
use crate::v311::cp_fixed_header::FixedHeader;
use crate::v311::variable::Variable;
use crate::v311::PacketId;
use crate::v311::{CPRemLen, CPType, ControlPacket, Flag};

pub struct PubrelPacketBuilder {
    packet: ControlPacket,
}

impl PubrelPacketBuilder {
    pub fn new(packet_id: &PacketId) -> Self {
        PubrelPacketBuilder {
            packet: ControlPacket {
                fixed_header: FixedHeader {
                    cp_type: CPType::Pubrel,
                    flag: Flag {
                        control_packet: CPType::Pubrel,
                        is_reserved: true,
                        bits: 0b0010,
                    },
                    remaining_length: CPRemLen::new(2),
                },
                variable: Variable::Pubrel(BasicVariable {
                    packet_id: packet_id.clone(),
                }),
            },
        }
    }

    pub fn build(self) -> ControlPacket {
        self.packet
    }
}
