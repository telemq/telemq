/// MQTT Control Packet type
///
/// Position: byte 1, bits 7-4.
///
/// Represented as a 4-bit unsigned value.
#[derive(Debug, PartialEq, Clone)]
pub enum CPType {
    /// Direction of flow: Client to Server.
    ///
    /// Description: Client request to connect to Server.
    Connect,

    /// Direction of flow: Server to Client.
    ///
    /// Description: Connect acknowledgment.
    Connack,

    /// Direction of flow: Client to Server OR Server to Client.
    ///
    /// Description: Publish message.
    Publish,

    /// Direction of flow: Client to Server OR Server to Client.
    ///
    /// Description: Publish acknowledgment.
    Puback,

    /// Direction of flow: Client to Server OR Server to Client.
    ///
    /// Description: Publish received (assured delivery part 1).
    Pubrec,

    /// Direction of flow: Client to Server OR Server to Client.
    ///
    /// Description: Publish received (assured delivery part 2).
    Pubrel,

    /// Direction of flow: Client to Server OR Server to Client.
    ///
    /// Description: Publish received (assured delivery part 3).
    Pubcomp,

    /// Direction of flow: Client to Server.
    ///
    /// Description: Client subscribe request.
    Subscribe,

    /// Direction of flow: Client to Server.
    ///
    /// Description: Subscribe acknowledgment.
    Suback,

    /// Direction of flow: Client to Server.
    ///
    /// Description: Unsubscribe request.
    Unsubscribe,

    /// Direction of flow: Server to Client.
    ///
    /// Description: Unsubscribe acknowledgment.
    Unsuback,

    /// Direction of flow: Client to Server.
    ///
    /// Description: PING request.
    Pingreq,

    /// Direction of flow: Server to Client.
    ///
    /// Description: PING response
    Pingresp,

    /// Direction of flow: Client to Server.
    ///
    /// Description: Client is disconnecting.
    Disconnect,
}

impl CPType {
    const CONNECT: u8 = 1;
    const CONNACK: u8 = 2;
    const PUBLISH: u8 = 3;
    const PUBACK: u8 = 4;
    const PUBREC: u8 = 5;
    const PUBREL: u8 = 6;
    const PUBCOMP: u8 = 7;
    const SUBSCRIBE: u8 = 8;
    const SUBACK: u8 = 9;
    const UNSUBSCRIBE: u8 = 10;
    const UNSUBACK: u8 = 11;
    const PINGREQ: u8 = 12;
    const PINGRESP: u8 = 13;
    const DISCONNECT: u8 = 14;

    const MASK: u8 = 0b11110000;

    /// It decodes a byte into `CPType` basing of first two bytes of a byte
    /// that was provided as an argument.
    pub fn decode(byte: &u8) -> ::std::io::Result<Self> {
        let cp_type = (byte & Self::MASK).rotate_right(4);
        match cp_type {
            Self::CONNECT => Ok(CPType::Connect),
            Self::CONNACK => Ok(CPType::Connack),
            Self::PUBLISH => Ok(CPType::Publish),
            Self::PUBACK => Ok(CPType::Puback),
            Self::PUBREC => Ok(CPType::Pubrec),
            Self::PUBREL => Ok(CPType::Pubrel),
            Self::PUBCOMP => Ok(CPType::Pubcomp),
            Self::SUBSCRIBE => Ok(CPType::Subscribe),
            Self::SUBACK => Ok(CPType::Suback),
            Self::UNSUBSCRIBE => Ok(CPType::Unsubscribe),
            Self::UNSUBACK => Ok(CPType::Unsuback),
            Self::PINGREQ => Ok(CPType::Pingreq),
            Self::PINGRESP => Ok(CPType::Pingresp),
            Self::DISCONNECT => Ok(CPType::Disconnect),
            _ => Err(::std::io::Error::new(
                ::std::io::ErrorKind::Other,
                "Unexpected Control Packet type",
            )),
        }
    }

    /// It encodes `CPType` into `u8`.
    pub fn encode(&self) -> ::std::io::Result<u8> {
        match *self {
            CPType::Connect => Ok(Self::CONNECT),
            CPType::Connack => Ok(Self::CONNACK),
            CPType::Publish => Ok(Self::PUBLISH),
            CPType::Puback => Ok(Self::PUBACK),
            CPType::Pubrec => Ok(Self::PUBREC),
            CPType::Pubrel => Ok(Self::PUBREL),
            CPType::Pubcomp => Ok(Self::PUBCOMP),
            CPType::Subscribe => Ok(Self::SUBSCRIBE),
            CPType::Suback => Ok(Self::SUBACK),
            CPType::Unsubscribe => Ok(Self::UNSUBSCRIBE),
            CPType::Unsuback => Ok(Self::UNSUBACK),
            CPType::Pingreq => Ok(Self::PINGREQ),
            CPType::Pingresp => Ok(Self::PINGRESP),
            CPType::Disconnect => Ok(Self::DISCONNECT),
        }
        .map(|v| v.rotate_left(4))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode() {
        assert_eq!(
            CPType::decode(&type_bits(1)).expect("Connect is expected"),
            CPType::Connect
        );
        assert_eq!(
            CPType::decode(&type_bits(2)).expect("Connack is expected"),
            CPType::Connack
        );
        assert_eq!(
            CPType::decode(&type_bits(3)).expect("Publish is expected"),
            CPType::Publish
        );
        assert_eq!(
            CPType::decode(&type_bits(4)).expect("Puback is expected"),
            CPType::Puback
        );
        assert_eq!(
            CPType::decode(&type_bits(5)).expect("Pubrec is expected"),
            CPType::Pubrec
        );
        assert_eq!(
            CPType::decode(&type_bits(6)).expect("Pubrel is expected"),
            CPType::Pubrel
        );
        assert_eq!(
            CPType::decode(&type_bits(7)).expect("Pubcomp is expected"),
            CPType::Pubcomp
        );
        assert_eq!(
            CPType::decode(&type_bits(8)).expect("Subscribe is expected"),
            CPType::Subscribe
        );
        assert_eq!(
            CPType::decode(&type_bits(9)).expect("Suback is expected"),
            CPType::Suback
        );
        assert_eq!(
            CPType::decode(&type_bits(10)).expect("Unsubscribe is expected"),
            CPType::Unsubscribe
        );
        assert_eq!(
            CPType::decode(&type_bits(11)).expect("Unsuback is expected"),
            CPType::Unsuback
        );
        assert_eq!(
            CPType::decode(&type_bits(12)).expect("Pingreq is expected"),
            CPType::Pingreq
        );
        assert_eq!(
            CPType::decode(&type_bits(13)).expect("Pingresp is expected"),
            CPType::Pingresp
        );
        assert_eq!(
            CPType::decode(&type_bits(14)).expect("Disconnect is expected"),
            CPType::Disconnect
        );
    }

    #[test]
    fn encode() {
        assert_eq!(CPType::Connect.encode().unwrap(), type_bits(1));
        assert_eq!(CPType::Connack.encode().unwrap(), type_bits(2));
        assert_eq!(CPType::Publish.encode().unwrap(), type_bits(3));
        assert_eq!(CPType::Puback.encode().unwrap(), type_bits(4));
        assert_eq!(CPType::Pubrec.encode().unwrap(), type_bits(5));
        assert_eq!(CPType::Pubrel.encode().unwrap(), type_bits(6));
        assert_eq!(CPType::Pubcomp.encode().unwrap(), type_bits(7));
        assert_eq!(CPType::Subscribe.encode().unwrap(), type_bits(8));
        assert_eq!(CPType::Suback.encode().unwrap(), type_bits(9));
        assert_eq!(CPType::Unsubscribe.encode().unwrap(), type_bits(10));
        assert_eq!(CPType::Unsuback.encode().unwrap(), type_bits(11));
        assert_eq!(CPType::Pingreq.encode().unwrap(), type_bits(12));
        assert_eq!(CPType::Pingresp.encode().unwrap(), type_bits(13));
        assert_eq!(CPType::Disconnect.encode().unwrap(), type_bits(14));
    }

    fn type_bits(t: u8) -> u8 {
        t.rotate_left(4)
    }
}
