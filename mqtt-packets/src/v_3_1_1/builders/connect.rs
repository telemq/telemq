use crate::v_3_1_1::cp_fixed_header::FixedHeader;
use crate::v_3_1_1::cp_flag::Flag;
use crate::v_3_1_1::cp_rem_len::CPRemLen;
use crate::v_3_1_1::cp_type::CPType;
use crate::v_3_1_1::ControlPacket;

use crate::v_3_1_1::connect::connect_flags::ConnectFlags;
use crate::v_3_1_1::connect::keep_alive::KeepAlive;
use crate::v_3_1_1::connect::protocol_level::ProtocolLevel;
use crate::v_3_1_1::connect::protocol_name::ProtocolName;
use crate::v_3_1_1::connect::variable::Variable as ConnectVariable;
use crate::v_3_1_1::variable::Variable;

/// Connack Control Packet builder. Default Control Packet is of type Connack,
/// with remaining_length 0, with `Flags::SessionNotPresent` flag and
/// with return code Accepted. All these values could be overrided by
/// builder's helper methods.
pub struct ConnectBuilder {
    control_packet: ControlPacket,
}

impl ConnectBuilder {
    /// `ConnackBuilder` constructor function that creates a new instance of the
    /// builder with default values.
    pub fn new(
        client_identifier: String,
        keep_alive_secs: u16,
        clean_session: bool,
        username: Option<String>,
        password: Option<String>,
    ) -> Self {
        let flag = Flag::with_type(CPType::Connect);
        let fixed_header = FixedHeader {
            flag: flag,
            cp_type: CPType::Connect,
            // will be overriden
            remaining_length: CPRemLen::new(0),
        };
        let mut connect_flags = ConnectFlags::new(0);
        connect_flags.set_clean_session(clean_session);
        connect_flags.set_username(username.is_some());
        connect_flags.set_password(password.is_some());
        let variable = Variable::Connect(ConnectVariable {
            // variable header
            protocol_name: ProtocolName::new(ProtocolName::SUPPORTED_PROTOCOL_NAME),
            protocol_level: ProtocolLevel::new(),
            connect_flags,
            keep_alive: KeepAlive::new(keep_alive_secs),
            // variable payload
            client_identifier,
            will_topic: None,
            will_message: None,
            username,
            password,
        });
        ConnectBuilder {
            control_packet: ControlPacket {
                fixed_header,
                variable,
            },
        }
    }

    /// It finalizes build process and returns resulting `ControlPacket`.
    pub fn build(self) -> ControlPacket {
        self.control_packet
    }
}
