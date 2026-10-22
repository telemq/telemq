use bytes::{BufMut, BytesMut};

use super::cp_flag::Flag;
use super::cp_rem_len::{CPRemLen, CPRemLenCodec};
use super::cp_type::CPType;

/// MQTT Control Packet fixed header structure.
#[derive(Debug, PartialEq, Clone)]
pub struct FixedHeader {
    /// Header flag.
    pub flag: Flag,

    // FIXME: duplicates Flag.control_packet?????
    /// A type of Control Packet.
    pub cp_type: CPType,

    /// MQTT Control Packet remaining length.
    pub remaining_length: CPRemLen,
}

/// `FixedHeader` Tokio codec. The codec is statefull, so in order to
/// reuse it few times it's neccessary to reset its state via
/// `codec.reset()`.
#[derive(Default, Debug)]
pub struct FixedHeaderCodec {
    type_decoded: Option<CPType>,
    flag_decoded: Option<Flag>,
    remaining_length_decoded: Option<CPRemLen>,
    remaining_length_codec: CPRemLenCodec,
}

impl FixedHeaderCodec {
    /// It resets internal state of the codec.
    pub fn reset(&mut self) {
        self.type_decoded = None;
        self.flag_decoded = None;
        self.remaining_length_decoded = None;
        self.remaining_length_codec.reset();
    }

    pub fn encode(
        &mut self,
        item: &FixedHeader,
        remaining_length: u32,
        dst: &mut BytesMut,
    ) -> Result<(), std::io::Error> {
        let type_bits = item.cp_type.encode()?;
        let flag_bits = item.flag.encode()?;
        // put a byte which is combined from type and flag bits
        dst.put_u8(type_bits | flag_bits);

        let mut remaining_length_codec: CPRemLenCodec = Default::default();

        remaining_length_codec.encode(&CPRemLen::new(remaining_length), dst)
    }

    pub fn decode(&mut self, src: &mut BytesMut) -> Result<Option<FixedHeader>, std::io::Error> {
        if self.type_decoded.is_none() && src.len() >= 1 {
            match src.split_to(1).first() {
                Some(b) => {
                    let cp_type = CPType::decode(b)?;
                    let cp_flag = Flag::decode(b, &cp_type)?;
                    self.type_decoded = Some(cp_type);
                    self.flag_decoded = Some(cp_flag);
                }
                _ => return Ok(None),
            };
        }

        if self.remaining_length_decoded.is_none() {
            if src.is_empty() {
                return Ok(None);
            }
            let remaining_length = self.remaining_length_codec.decode(src)?;
            self.remaining_length_decoded = remaining_length;
        }

        Ok(self.build())
    }

    fn build(&mut self) -> Option<FixedHeader> {
        if self.type_decoded.is_some()
            && self.flag_decoded.is_some()
            && self.remaining_length_decoded.is_some()
        {
            // following unwraps are safe as we've just checked that each of
            // options is some
            Some(FixedHeader {
                flag: self.flag_decoded.take().unwrap(),
                cp_type: self.type_decoded.take().unwrap(),
                remaining_length: self.remaining_length_decoded.take().unwrap(),
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_header_codec_default() {
        let codec: FixedHeaderCodec = Default::default();
        assert_eq!(codec.type_decoded, None);
        assert_eq!(codec.flag_decoded, None);
        assert_eq!(codec.remaining_length_decoded, None);
    }

    #[test]
    fn test_fixed_header_codec_reset() {
        let mut codec: FixedHeaderCodec = Default::default();
        codec.type_decoded = Some(CPType::Publish);
        codec.flag_decoded = Some(Flag {
            control_packet: CPType::Publish,
            is_reserved: true,
            bits: 0,
        });
        codec.remaining_length_decoded = Some(CPRemLen::new(1));
        codec.reset();
        assert_eq!(codec.type_decoded, None);
        assert_eq!(codec.flag_decoded, None);
        assert_eq!(codec.remaining_length_decoded, None);
    }

    #[test]
    fn test_fixed_header_codec_build() {
        let mut codec: FixedHeaderCodec = Default::default();

        codec.type_decoded = Some(CPType::Publish);
        assert!(
            codec.build().is_none(),
            "Should not be ready to build FixedHeader"
        );

        codec.flag_decoded = Some(Flag {
            control_packet: CPType::Publish,
            is_reserved: true,
            bits: 0,
        });
        assert!(
            codec.build().is_none(),
            "Should not be ready to build FixedHeader"
        );

        codec.remaining_length_decoded = Some(CPRemLen::new(1));
        assert_eq!(
            codec.build().unwrap(),
            FixedHeader {
                cp_type: CPType::Publish,
                flag: Flag {
                    control_packet: CPType::Publish,
                    is_reserved: true,
                    bits: 0,
                },
                remaining_length: CPRemLen::new(1),
            }
        );
    }

    #[test]
    fn test_cp_fixed_header_decode() {
        let mut codec: FixedHeaderCodec = Default::default();
        let mut buf = BytesMut::from(vec![(3 as u8).rotate_left(4) | 1, 0xFF, 0x7F].as_slice());

        // immitate loop turns
        codec
            .decode(&mut buf)
            .expect("This turn should end with Ok");
        let actual_fixed_header = codec
            .decode(&mut buf)
            .expect("Fixed header should be decoded without errors")
            .unwrap();
        let expected_flag_fixed_header = FixedHeader {
            cp_type: CPType::Publish,
            flag: Flag {
                control_packet: CPType::Publish,
                is_reserved: false,
                bits: 1,
            },
            remaining_length: CPRemLen::new(16_383),
        };
        assert_eq!(
            actual_fixed_header, expected_flag_fixed_header,
            "Expected and actual header don't match"
        );
    }
}
