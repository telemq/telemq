use bytes::{BufMut, BytesMut};

/// Connect Acknowledge Flags.
#[derive(Debug, PartialEq, Clone)]
pub enum Flags {
    SessionPresent,
    SessionNotPresent,
}

impl Flags {
    const SESSION_PRESENT_BYTE: u8 = 1;
    const SESSION_NOT_PRESENT_BYTE: u8 = 0;
    const FLAGS_BYTES_LEN: usize = 1;

    fn as_byte(&self) -> u8 {
        match *self {
            Flags::SessionPresent => Self::SESSION_PRESENT_BYTE,
            Flags::SessionNotPresent => Self::SESSION_NOT_PRESENT_BYTE,
        }
    }
}

/// Connect Acknowledge Flags Tokio codec.
pub struct FlagsCodec;

impl FlagsCodec {
    pub fn new() -> Self {
        FlagsCodec {}
    }
}

impl FlagsCodec {
    pub fn encode(&mut self, item: &Flags, dst: &mut BytesMut) -> Result<(), std::io::Error> {
        Ok(dst.put_u8(item.as_byte()))
    }

    pub fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Flags>, std::io::Error> {
        if src.len() < Flags::FLAGS_BYTES_LEN {
            return Ok(None);
        }
        let byte = src.split_to(Flags::FLAGS_BYTES_LEN);

        if byte.len() == Flags::FLAGS_BYTES_LEN {
            let flags = match byte[0] {
                Flags::SESSION_PRESENT_BYTE => Flags::SessionPresent,
                Flags::SESSION_NOT_PRESENT_BYTE => Flags::SessionNotPresent,
                _ => {
                    return Err(::std::io::Error::new(
                        ::std::io::ErrorKind::Other,
                        "Unexpected Connect Acknowledge Flags byte",
                    ));
                }
            };
            Ok(Some(flags))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flags_encode() {
        let mut codec = FlagsCodec {};

        // encode session present flags
        {
            let mut buf = BytesMut::new();
            codec
                .encode(&Flags::SessionPresent, &mut buf)
                .expect("Should encode SessionPresent flags without errors");
            assert_eq!(buf, vec![Flags::SESSION_PRESENT_BYTE]);
        }

        // encode session not present flags
        {
            let mut buf = BytesMut::new();
            codec
                .encode(&Flags::SessionNotPresent, &mut buf)
                .expect("Should encode SessionNotPresent flags without errors");
            assert_eq!(buf, vec![Flags::SESSION_NOT_PRESENT_BYTE]);
        }
    }

    #[test]
    fn test_flags_decode() {
        let mut codec = FlagsCodec {};

        // decode session present flags
        {
            let mut buf = BytesMut::new();
            assert!(
                codec
                    .decode(&mut buf)
                    .expect("Should decode SessionPresent flags without errors")
                    .is_none(),
                "Shuld keep waiting for buffer if number of bytes is not enought for decoding"
            );
            buf.put_u8(Flags::SESSION_PRESENT_BYTE);
            assert_eq!(
                codec
                    .decode(&mut buf)
                    .expect("Should decode SessionPresent flags without errors")
                    .expect("Should return some Flags"),
                Flags::SessionPresent
            );
        }

        // decode session not present flags
        {
            let mut buf = BytesMut::new();
            assert!(
                codec
                    .decode(&mut buf)
                    .expect("Should decode SessionNotPresent flags without errors")
                    .is_none(),
                "Shuld keep waiting for buffer if number of bytes is not enought for decoding"
            );
            buf.put_u8(Flags::SESSION_NOT_PRESENT_BYTE);
            assert_eq!(
                codec
                    .decode(&mut buf)
                    .expect("Should decode SessionNotPresent flags without errors")
                    .expect("Should return some Flags"),
                Flags::SessionNotPresent
            );
        }

        // decode unexpected flags
        {
            let mut buf = BytesMut::new();
            assert!(
                codec
                    .decode(&mut buf)
                    .expect("Should decode an empty buffer without errors")
                    .is_none(),
                "Shuld keep waiting for buffer if number of bytes is not enought for decoding"
            );
            buf.put_u8(100);
            assert!(
                codec.decode(&mut buf).is_err(),
                "Should return an error if bytes are not expected for Flags"
            );
        }
    }
}
