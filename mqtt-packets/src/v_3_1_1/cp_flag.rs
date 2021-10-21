use super::cp_type::CPType;

/// The remaining bits [3-0] of byte 1 in the fixed header contain flags
/// specific to each MQTT Control Packet type.
#[derive(Debug, PartialEq, Clone)]
pub struct Flag {
    /// The property `cp` represents Control Packet type.
    pub control_packet: CPType,

    /// It indicates if current flag is reserved or not.
    pub is_reserved: bool,

    /// Flag bits.
    pub bits: u8,
}

impl Flag {
    const CONNECT: Flag = Flag {
        control_packet: CPType::Connect,
        is_reserved: true,
        bits: 0,
    };

    const CONNACK: Flag = Flag {
        control_packet: CPType::Connack,
        is_reserved: true,
        bits: 0,
    };

    const PUBLISH: Flag = Flag {
        control_packet: CPType::Publish,
        is_reserved: false,
        bits: 0,
    };

    const PUBACK: Flag = Flag {
        control_packet: CPType::Puback,
        is_reserved: true,
        bits: 0,
    };

    const PUBREC: Flag = Flag {
        control_packet: CPType::Pubrec,
        is_reserved: true,
        bits: 0,
    };

    const PUBREL: Flag = Flag {
        control_packet: CPType::Pubrel,
        is_reserved: true,
        bits: 2,
    };

    const PUBCOMP: Flag = Flag {
        control_packet: CPType::Pubcomp,
        is_reserved: true,
        bits: 0,
    };

    const SUBSCRIBE: Flag = Flag {
        control_packet: CPType::Subscribe,
        is_reserved: true,
        bits: 2,
    };

    const SUBACK: Flag = Flag {
        control_packet: CPType::Suback,
        is_reserved: true,
        bits: 0,
    };

    const UNSUBSCRIBE: Flag = Flag {
        control_packet: CPType::Unsubscribe,
        is_reserved: true,
        bits: 2,
    };

    const UNSUBACK: Flag = Flag {
        control_packet: CPType::Unsuback,
        is_reserved: true,
        bits: 0,
    };

    const PINGREQ: Flag = Flag {
        control_packet: CPType::Pingreq,
        is_reserved: true,
        bits: 0,
    };

    const PINGRESP: Flag = Flag {
        control_packet: CPType::Pingresp,
        is_reserved: true,
        bits: 0,
    };

    const DISCONNECT: Flag = Flag {
        control_packet: CPType::Disconnect,
        is_reserved: true,
        bits: 0,
    };

    const MASK: u8 = 0b00001111;
    const QOS_MASK: u8 = 0b00000110;

    /// It creates a new flag of provided control packet type.
    // TODO: implement rest of branches and add related tests.
    pub fn with_type(cp_type: CPType) -> Flag {
        match cp_type {
            CPType::Connect => Self::CONNECT,
            CPType::Connack => Self::CONNACK,
            _ => unimplemented!(),
        }
    }

    /// Try to return a value of QoS flag bits. If it's neither of 0, 1, 2
    /// then return None. This case should be interpreted as reserved and
    /// should not be used according to the spec.
    pub fn try_qos(flag: &Flag) -> Option<u8> {
        let value = (flag.bits & Self::QOS_MASK).rotate_right(1);
        if value < 3 {
            Some(value)
        } else {
            None
        }
    }

    /// It decodes a byte into a Control Packet flag assuming that the Packet
    /// is of type `cp_type`. It returns an error if a `Flag` is reserved according
    /// to a spec and provided bits don't match an expected reserved value.
    pub fn decode(bits: &u8, cp_type: &CPType) -> ::std::io::Result<Flag> {
        let masked_bits = bits & Self::MASK;
        let mut flag = match *cp_type {
            CPType::Connect => Self::CONNECT,
            CPType::Connack => Self::CONNACK,
            CPType::Publish => Self::PUBLISH,
            CPType::Puback => Self::PUBACK,
            CPType::Pubrec => Self::PUBREC,
            CPType::Pubrel => Self::PUBREL,
            CPType::Pubcomp => Self::PUBCOMP,
            CPType::Subscribe => Self::SUBSCRIBE,
            CPType::Suback => Self::SUBACK,
            CPType::Unsubscribe => Self::UNSUBSCRIBE,
            CPType::Unsuback => Self::UNSUBACK,
            CPType::Pingreq => Self::PINGREQ,
            CPType::Pingresp => Self::PINGRESP,
            CPType::Disconnect => Self::DISCONNECT,
        };

        if !flag.is_reserved || masked_bits == flag.bits {
            flag.bits = masked_bits;
            Ok(flag)
        } else {
            Err(::std::io::Error::new(
                ::std::io::ErrorKind::Other,
                "Unexpected flag: missmatched bits for reserved flag",
            ))
        }
    }

    /// It encodes a `Flag` into bits.
    pub fn encode(&self) -> ::std::io::Result<u8> {
        Ok(self.bits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v_3_1_1::cp_type::CPType;

    #[test]
    fn decode_non_reserved() {
        let cp_type = CPType::Publish;
        let bits = 0b01001010;
        let expected_flag = Flag {
            control_packet: CPType::Publish,
            is_reserved: false,
            bits: 0b00001010,
        };

        assert_eq!(Flag::decode(&bits, &cp_type).unwrap(), expected_flag)
    }

    #[test]
    fn decode_reserved_non_expected() {
        let cp_type = CPType::Connect;
        let bits = 0b00001010;

        assert!(Flag::decode(&bits, &cp_type).is_err())
    }

    #[test]
    fn decode_reserved() {
        assert!(Flag::decode(&0b0000000, &CPType::Connect).is_ok());
        assert!(Flag::decode(&0b0000000, &CPType::Connack).is_ok());
        assert!(Flag::decode(&0b0000000, &CPType::Puback).is_ok());
        assert!(Flag::decode(&0b0000000, &CPType::Pubrec).is_ok());
        assert!(Flag::decode(&0b0000010, &CPType::Pubrel).is_ok());
        assert!(Flag::decode(&0b0000000, &CPType::Pubcomp).is_ok());
        assert!(Flag::decode(&0b0000010, &CPType::Subscribe).is_ok());
        assert!(Flag::decode(&0b0000000, &CPType::Suback).is_ok());
        assert!(Flag::decode(&0b0000010, &CPType::Unsubscribe).is_ok());
        assert!(Flag::decode(&0b0000000, &CPType::Unsuback).is_ok());
        assert!(Flag::decode(&0b0000000, &CPType::Pingreq).is_ok());
        assert!(Flag::decode(&0b0000000, &CPType::Pingresp).is_ok());
        assert!(Flag::decode(&0b0000000, &CPType::Disconnect).is_ok());
    }

    #[test]
    fn create_with_type() {
        {
            let flag = Flag::with_type(CPType::Connect);
            assert_eq!(flag, Flag::CONNECT, "should create a flag of type Connect");
        }
        {
            let flag = Flag::with_type(CPType::Connack);
            assert_eq!(flag, Flag::CONNACK, "should create a flag of type Connack");
        }
    }
}
