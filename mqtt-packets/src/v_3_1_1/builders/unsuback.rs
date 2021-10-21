use crate::v_3_1_1::basic_variable::Variable as UnsubackVariable;
use crate::v_3_1_1::cp_fixed_header::FixedHeader;
use crate::v_3_1_1::packet_id::{PacketId, PACKET_ID_LEN};
use crate::v_3_1_1::variable::Variable;
use crate::v_3_1_1::Flag;
use crate::v_3_1_1::{CPRemLen, CPType, ControlPacket};

pub struct UnsubackPacketBuilder {
    packet: ControlPacket,
}

impl UnsubackPacketBuilder {
    pub fn new(packet_id: PacketId) -> Self {
        UnsubackPacketBuilder {
            packet: ControlPacket {
                fixed_header: FixedHeader {
                    cp_type: CPType::Unsuback,
                    flag: Flag {
                        control_packet: CPType::Unsuback,
                        is_reserved: true,
                        bits: 0,
                    },
                    remaining_length: CPRemLen::new(0),
                },
                variable: Variable::Unsuback(UnsubackVariable { packet_id }),
            },
        }
    }

    pub fn with_packet_id(mut self, packet_id: Vec<u8>) -> Self {
        if let Variable::Unsuback(ref mut variable) = self.packet.variable {
            variable.packet_id = packet_id;
        }

        self
    }

    pub fn build(mut self) -> ControlPacket {
        self.set_remaining_length();
        self.packet
    }

    fn set_remaining_length(&mut self) {
        let rem_len = PACKET_ID_LEN;

        self.packet.fixed_header.remaining_length = CPRemLen::new(rem_len as u32);
    }
}
