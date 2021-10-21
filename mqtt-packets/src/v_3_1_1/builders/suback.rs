use crate::v_3_1_1::cp_fixed_header::FixedHeader;
use crate::v_3_1_1::suback::{return_code::ReturnCode, variable::Variable as SubackVariable};
use crate::v_3_1_1::variable::Variable;
use crate::v_3_1_1::{CPRemLen, CPType, ControlPacket, Flag};
use crate::v_3_1_1::{PacketId, PACKET_ID_LEN};

pub struct SubackPacketBuilder {
    packet: ControlPacket,
}

impl SubackPacketBuilder {
    pub fn new(packet_id: PacketId) -> Self {
        SubackPacketBuilder {
            packet: ControlPacket {
                fixed_header: FixedHeader {
                    cp_type: CPType::Suback,
                    flag: Flag {
                        control_packet: CPType::Suback,
                        is_reserved: true,
                        bits: 0,
                    },
                    remaining_length: CPRemLen::new(0),
                },
                variable: Variable::Suback(SubackVariable {
                    packet_id,
                    return_codes: vec![],
                }),
            },
        }
    }

    pub fn with_packet_id(mut self, packet_id: Vec<u8>) -> Self {
        if let Variable::Suback(ref mut variable) = self.packet.variable {
            variable.packet_id = packet_id;
        }

        self
    }

    pub fn with_return_codes(mut self, return_codes: Vec<ReturnCode>) -> Self {
        if let Variable::Suback(ref mut variable) = self.packet.variable {
            variable.return_codes = return_codes;
        }

        self
    }

    pub fn build(mut self) -> ControlPacket {
        self.set_remaining_length();
        self.packet
    }

    fn set_remaining_length(&mut self) {
        let mut rem_len = 0usize;

        rem_len += PACKET_ID_LEN;

        if let Variable::Suback(ref variable) = self.packet.variable {
            rem_len += variable.return_codes.len();
        }

        self.packet.fixed_header.remaining_length = CPRemLen::new(rem_len as u32);
    }
}
