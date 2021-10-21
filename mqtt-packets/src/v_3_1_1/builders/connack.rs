use crate::v_3_1_1::cp_fixed_header::FixedHeader;
use crate::v_3_1_1::cp_flag::Flag;
use crate::v_3_1_1::cp_rem_len::CPRemLen;
use crate::v_3_1_1::cp_type::CPType;
use crate::v_3_1_1::ControlPacket;

use crate::v_3_1_1::connack::flags::Flags;
use crate::v_3_1_1::connack::return_code::ReturnCode;
use crate::v_3_1_1::connack::variable::Variable as ConnackVariable;
use crate::v_3_1_1::variable::Variable;

/// Connack Control Packet builder. Default Control Packet is of type Connack,
/// with remaining_length 0, with `Flags::SessionNotPresent` flag and
/// with return code Accepted. All these values could be overrided by
/// builder's helper methods.
pub struct ConnackBuilder {
    control_packet: ControlPacket,
}

impl ConnackBuilder {
    /// `ConnackBuilder` constructor function that creates a new instance of the
    /// builder with default values.
    pub fn new() -> Self {
        let flag = Flag::with_type(CPType::Connack);
        let fixed_header = FixedHeader {
            flag: flag,
            cp_type: CPType::Connack,
            // for CONACK package it's always 2
            remaining_length: CPRemLen::new(2),
        };
        let variable = Variable::Connack(ConnackVariable::create_with(
            Flags::SessionNotPresent,
            ReturnCode::Accepted,
        ));
        ConnackBuilder {
            control_packet: ControlPacket {
                fixed_header,
                variable,
            },
        }
    }

    /// It allows re-define control packet's session present flag.
    pub fn with_session_presented(mut self, session_present: bool) -> Self {
        let session_flag = if session_present {
            Flags::SessionPresent
        } else {
            Flags::SessionNotPresent
        };
        match self.control_packet.variable {
            Variable::Connack(ref mut v) => {
                v.flags = session_flag;
            }
            // we should not have cases when we have non-connack variable header
            // inside connack builder
            _ => unreachable!(),
        }
        self
    }

    /// It allows re-define return code.
    pub fn with_return_code(mut self, return_code: ReturnCode) -> Self {
        match self.control_packet.variable {
            Variable::Connack(ref mut v) => {
                v.return_code = return_code;
            }
            // we should not have cases when we have non-connack variable header
            // inside connack builder
            _ => unreachable!(),
        }
        self
    }

    /// It finalizes build process and returns resulting `ControlPacket`.
    pub fn build(self) -> ControlPacket {
        self.control_packet
    }
}
