use crate::v_3_1_1::cp_fixed_header::FixedHeader;
use crate::v_3_1_1::variable::Variable;
use crate::v_3_1_1::Flag;
use crate::v_3_1_1::{CPRemLen, CPType, ControlPacket};

pub struct PingrespPacketBuilder {
    packet: ControlPacket,
}

impl PingrespPacketBuilder {
    pub fn new() -> Self {
        PingrespPacketBuilder {
            packet: ControlPacket {
                fixed_header: FixedHeader {
                    cp_type: CPType::Pingresp,
                    flag: Flag {
                        control_packet: CPType::Pingresp,
                        is_reserved: true,
                        bits: 0,
                    },
                    remaining_length: CPRemLen::new(0),
                },
                variable: Variable::Pingresp,
            },
        }
    }

    pub fn build(self) -> ControlPacket {
        self.packet
    }
}
