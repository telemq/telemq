use crate::v311::basic_variable::Variable as BasicVariable;
use crate::v311::cp_fixed_header::FixedHeader;
use crate::v311::variable::Variable;
use crate::v311::PacketId;
use crate::v311::{CPRemLen, CPType, ControlPacket, Flag};

pub struct PubrecPacketBuilder {
    packet: ControlPacket,
}

impl PubrecPacketBuilder {
    pub fn new(packet_id: &PacketId) -> Self {
        PubrecPacketBuilder {
            packet: ControlPacket {
                fixed_header: FixedHeader {
                    cp_type: CPType::Pubrec,
                    flag: Flag {
                        control_packet: CPType::Pubrec,
                        is_reserved: true,
                        bits: 0,
                    },
                    remaining_length: CPRemLen::new(2),
                },
                variable: Variable::Pubrec(BasicVariable {
                    packet_id: packet_id.clone(),
                }),
            },
        }
    }

    pub fn build(self) -> ControlPacket {
        self.packet
    }
}
