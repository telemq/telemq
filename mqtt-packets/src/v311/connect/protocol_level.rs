use bytes::{BufMut, BytesMut};

/// The 8 bit unsigned value that represents the revision level of the protocol
/// used by the Client. The value of the Protocol Level field for the version 3.1.1
/// of the protocol is 4 (0x04).
#[derive(Debug, PartialEq, Clone)]
pub struct ProtocolLevel(u8);

impl ProtocolLevel {
    pub const SUPPORTED_LEVEL: u8 = 4;

    /// `ProtocolLevel` constructor function. It always returns protocol level 4.
    pub fn new() -> ProtocolLevel {
        ProtocolLevel(Self::SUPPORTED_LEVEL)
    }

    /// Return underlaying value as a reference
    pub fn as_value(&self) -> u8 {
        self.0
    }
}

impl Default for ProtocolLevel {
    fn default() -> ProtocolLevel {
        ProtocolLevel::new()
    }
}

/// `ProtocolLevel` Tokio codec.
pub struct ProtocolLevelCodec;

impl ProtocolLevelCodec {
    const PROTOCOL_LEVEL_LEN: usize = 1;

    pub fn new() -> ProtocolLevelCodec {
        ProtocolLevelCodec {}
    }

    pub fn encode(
        &mut self,
        item: &ProtocolLevel,
        dst: &mut BytesMut,
    ) -> Result<(), std::io::Error> {
        dst.put_u8(item.0);

        Ok(())
    }

    pub fn decode(&mut self, src: &mut BytesMut) -> Result<Option<ProtocolLevel>, std::io::Error> {
        if src.len() < Self::PROTOCOL_LEVEL_LEN {
            return Ok(None);
        }
        let level_bits = src.split_to(Self::PROTOCOL_LEVEL_LEN);
        Ok(level_bits.first().map(|b| ProtocolLevel(*b)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_level_new() {
        assert_eq!(ProtocolLevel::new(), ProtocolLevel(4));
    }

    #[test]
    fn test_protocol_level_default() {
        let default: ProtocolLevel = Default::default();
        assert_eq!(default, ProtocolLevel(4));
    }

    #[test]
    fn test_protocol_level_encode() {
        let mut buf = BytesMut::new();
        let mut codec = ProtocolLevelCodec::new();
        codec
            .encode(&ProtocolLevel::new(), &mut buf)
            .expect("Should encode Protocol Level without errors");
        assert_eq!(buf.to_vec(), vec![4])
    }

    #[test]
    fn test_protocol_level_decode() {
        let mut buf = BytesMut::from(vec![4].as_slice());
        let mut codec = ProtocolLevelCodec::new();
        let level_opt = codec
            .decode(&mut buf)
            .expect("Should decode Protocol Level without errors");
        assert_eq!(level_opt.unwrap(), ProtocolLevel(4));
    }
}
