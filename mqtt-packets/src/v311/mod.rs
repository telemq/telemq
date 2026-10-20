pub mod builders;
pub mod topic;
pub mod utils;

pub mod connack;
pub mod connect;
pub mod cp_fixed_header;
pub mod publish;
pub mod suback;
pub mod subscribe;
pub mod unsubscribe;
pub mod variable;

mod basic_variable;
mod cp_flag;
mod cp_qos;
mod cp_rem_len;
mod cp_type;
mod packet_id;

use bytes::BytesMut;

use self::cp_fixed_header::{FixedHeader, FixedHeaderCodec};
pub use self::cp_flag::Flag;
pub use self::cp_qos::QoS;
pub use self::cp_rem_len::CPRemLen;
pub use self::cp_type::CPType;
pub use self::packet_id::{PacketId, PACKET_ID_LEN};
use self::variable::{Variable, VariableCodec};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A structure which represents MQTT Control Packet
#[derive(Debug, Clone)]
pub struct ControlPacket {
    pub fixed_header: FixedHeader,
    pub variable: Variable,
}

impl Serialize for ControlPacket {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut buf = BytesMut::new();
        let mut codec = ControlPacketCodec::new();
        codec.inner_encode(self, &mut buf).unwrap();
        serializer.serialize_bytes(buf.to_vec().as_slice())
    }
}

struct Bytes;

impl<'vi> serde::de::Visitor<'vi> for Bytes {
    type Value = Vec<u8>;

    fn expecting(&self, formatter: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(formatter, "bytes")
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(v.to_vec())
    }
}

impl<'de> Deserialize<'de> for ControlPacket {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let inner_bytes = Bytes {};
        let bytes = deserializer.deserialize_bytes(inner_bytes)?;
        let mut bytes_buf = BytesMut::from(bytes.as_slice());
        let mut codec = ControlPacketCodec::new();
        Ok(codec.inner_decode(&mut bytes_buf).unwrap().unwrap())
    }
}

/// `ControlPacket` Tokio codec.
pub struct ControlPacketCodec {
    fixed_header: Option<FixedHeader>,
    fixed_header_codec: FixedHeaderCodec,
    variable: Option<Variable>,
    variable_codec: Option<VariableCodec>,
}

impl tokio_util::codec::Decoder for ControlPacketCodec {
    type Item = ControlPacket;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<ControlPacket>, std::io::Error> {
        self.inner_decode(src)
    }
}

impl tokio_util::codec::Encoder<&ControlPacket> for ControlPacketCodec {
    type Error = std::io::Error;

    fn encode(&mut self, item: &ControlPacket, dst: &mut BytesMut) -> Result<(), Self::Error> {
        self.inner_encode(item, dst)
    }
}

impl ControlPacketCodec {
    /// `ControlPacketCodec` constructor function.
    pub fn new() -> Self {
        ControlPacketCodec {
            fixed_header: None,
            fixed_header_codec: Default::default(),
            variable: None,
            variable_codec: None,
        }
    }

    pub fn inner_encode(
        &mut self,
        item: &ControlPacket,
        dst: &mut BytesMut,
    ) -> Result<(), std::io::Error> {
        let mut buf = BytesMut::new();
        let mut variable_codec = VariableCodec::create(&item.fixed_header.flag)?;
        variable_codec.encode(&item.variable, &mut buf)?;
        self.fixed_header_codec
            .encode(&item.fixed_header, buf.len() as u32, dst)?;
        dst.extend_from_slice(&buf);
        self.reset();

        Ok(())
    }

    pub fn inner_decode(
        &mut self,
        src: &mut BytesMut,
    ) -> Result<Option<ControlPacket>, std::io::Error> {
        if self.can_decode_fixed_header() {
            self.fixed_header = self.fixed_header_codec.decode(src)?;

            // still not decoded, keep waiting
            if self.can_decode_fixed_header() {
                return Ok(None);
            } else {
                let fixed_header_ref = self.fixed_header.as_ref();

                self.variable_codec = match fixed_header_ref {
                    Some(header) => Some(VariableCodec::create(&header.flag)?),
                    None => None,
                };
            }
        }

        let fixed_header = self.fixed_header.take().unwrap();

        let remaining_length = fixed_header.remaining_length.as_value() as usize;

        if src.len() < remaining_length {
            self.fixed_header = Some(fixed_header);
            return Ok(None);
        }

        let mut rest_bytes = src.split_to(remaining_length);
        self.variable = match self.variable_codec.as_mut() {
            Some(ref mut codec) => codec.decode(&mut rest_bytes)?,
            None => None,
        };

        // This check should not be needed, but it was done to log potential bugs
        if self.variable.is_some() {
            let control_packet = ControlPacket {
                fixed_header,
                variable: self.variable.take().unwrap(),
            };
            self.reset();
            Ok(Some(control_packet))
        } else {
            Err(std::io::Error::new(std::io::ErrorKind::Other, "Codec Error: fixed header and variable should have some value, but some/both of them is empty"))
        }
    }

    fn reset(&mut self) {
        *self = ControlPacketCodec::new();
    }

    fn can_decode_fixed_header(&self) -> bool {
        self.fixed_header.is_none()
    }
}
