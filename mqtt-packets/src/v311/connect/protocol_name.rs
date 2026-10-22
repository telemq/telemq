use bytes::{BufMut, BytesMut};
use std::{convert::TryInto, io};

/// MQTT Connect protocol name struct.
#[derive(Debug, PartialEq, Clone)]
pub struct ProtocolName(String);

impl ProtocolName {
    pub const SUPPORTED_PROTOCOL_NAME: &'static str = "MQTT";
    const PROTCOL_NAME_LEN: u16 = 4;
    const LEN_LEN: u16 = 2;

    /// Constructor function which accepts `name` as a type
    /// which has implementation of `ToString` trait.
    pub fn new<T: ToString>(name: T) -> ProtocolName {
        ProtocolName(name.to_string())
    }

    /// The method that returns a protocol name as a `&str`.
    pub fn as_value<'a>(&'a self) -> &'a str {
        self.0.as_str()
    }
}

pub struct ProtocolNameCodec;

impl ProtocolNameCodec {
    pub fn new() -> ProtocolNameCodec {
        ProtocolNameCodec {}
    }

    pub fn encode(&mut self, item: &ProtocolName, dst: &mut BytesMut) -> Result<(), io::Error> {
        if item.0.len() as u16 > ProtocolName::PROTCOL_NAME_LEN {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Connect: cannot encode protocol name, the name is too long",
            ));
        }
        dst.put_u16(ProtocolName::PROTCOL_NAME_LEN);
        dst.put_slice(item.0.as_bytes());

        Ok(())
    }

    pub fn decode(&mut self, dst: &mut BytesMut) -> Result<Option<ProtocolName>, io::Error> {
        if (dst.len() as u16) < ProtocolName::PROTCOL_NAME_LEN + ProtocolName::LEN_LEN {
            return Ok(None);
        }

        let length_bits = dst.split_to(ProtocolName::LEN_LEN as usize);

        let length = u16::from_be_bytes(length_bits.to_vec().try_into().map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "Protocol Name: unable to convert protocol name length to u16",
            )
        })?);
        if length != ProtocolName::PROTCOL_NAME_LEN {
            return Err(::std::io::Error::new(
                ::std::io::ErrorKind::Other,
                format!(
                    "Protocol Name: protocol name length {:?} is unacceptable, {} is expected",
                    length_bits,
                    ProtocolName::PROTCOL_NAME_LEN
                ),
            ));
        }

        let name_bits = dst.split_to(ProtocolName::PROTCOL_NAME_LEN as usize);
        let name = String::from_utf8_lossy(&name_bits);

        Ok(Some(ProtocolName::new(name)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_name_new() {
        assert_eq!(ProtocolName::new("MQTT").as_value(), "MQTT");
    }

    #[test]
    fn test_protocol_name_encode() {
        let mut codec = ProtocolNameCodec::new();
        let protocol_name = ProtocolName::new("MQTT");
        let mut buf = BytesMut::new();
        codec
            .encode(&protocol_name, &mut buf)
            .expect("Protocol name codec should write encoded name into a buffer withou errors");
        let mut expected = vec![0, 4];
        expected.append(&mut "MQTT".to_string().into_bytes());
        assert_eq!(buf.to_vec(), expected);
    }

    #[test]
    fn test_protocol_name_decode() {
        // let mut codec = ProtocolNameCodec::new();
        // let mut buf = BytesMut::new();
        // buf.put(vec![0, 4]);
        // buf.put_slice("MQTT".to_string().into_bytes().as_slice());
        // let res = codec
        //     .decode(&mut buf)
        //     .expect("Protocol name codec should read bits from a buffer withou errors");
        // assert_eq!(res.expect("Should be Some(name)").0, "MQTT")
    }
}
