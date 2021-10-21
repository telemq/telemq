use bytes::{BufMut, BytesMut};

/// Control Packet remainig length.
#[derive(Debug, PartialEq, Clone)]
pub struct CPRemLen(u32);

impl CPRemLen {
    /// Constructor function which takes length value as its argument.
    pub fn new(value: u32) -> CPRemLen {
        CPRemLen(value)
    }

    /// Converts `CPRemLen` to its internal value.
    pub fn to_value(self) -> u32 {
        self.0
    }

    /// Converts `CPRemLen` to its internal value.
    pub fn as_value(&self) -> u32 {
        self.0
    }
}

/// Tokio codec for CPRemLen (Control Packet remainig length).
/// In case of decoding bites into `CPRemLen` it
/// is a statefull codec. So if you need decode remaining length few times
/// in a row, after each turn it's neccessary to reset a state via `codec.reset()`.
#[derive(Debug)]
pub struct CPRemLenCodec {
    value: u32,
    multiplier: u32,
}

impl CPRemLenCodec {
    /// It resets an internal state of the coded to initial one.
    pub fn reset(&mut self) {
        self.value = 0;
        self.multiplier = 1;
    }

    pub fn encode(&mut self, item: &CPRemLen, dst: &mut BytesMut) -> Result<(), std::io::Error> {
        // FIXME: can it be avoided
        let mut n = item.clone();
        loop {
            let mut encoded_byte = (n.0 % 128) as u8;
            n.0 /= 128;

            if n.0 > 0 {
                encoded_byte |= 128;
            }

            dst.put_u8(encoded_byte);

            if n.0 == 0 {
                break;
            }
        }

        Ok(())
    }

    pub fn decode(&mut self, src: &mut BytesMut) -> Result<Option<CPRemLen>, std::io::Error> {
        if src.len() < 1 {
            return Ok(None);
        }
        // remove first byte from src buffer and return it as a new BytesMut
        let first_buf = src.split_to(1);

        // get encoded byte as a first element of first_buf,
        let encoded_byte = match first_buf.first() {
            Some(f) => f,
            // otherwise keep waiting
            _ => return Ok(None),
        };

        self.value += self.multiplier * (encoded_byte & 127) as u32;
        self.multiplier *= 128;

        if encoded_byte & 128 == 0 {
            return Ok(Some(CPRemLen(self.value)));
        }

        if self.multiplier > 128 * 128 * 128 {
            return Err(::std::io::Error::new(
                ::std::io::ErrorKind::Other,
                "Control Packet: malformed remaining length",
            ));
        }

        Ok(None)
    }
}

impl Default for CPRemLenCodec {
    fn default() -> CPRemLenCodec {
        CPRemLenCodec {
            value: 0,
            multiplier: 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cp_rem_len_new() {
        assert_eq!((CPRemLen::new(1)).0, 1);
    }

    #[test]
    fn test_cp_rem_len_to_value() {
        assert_eq!((CPRemLen::new(1)).to_value(), 1);
    }

    #[test]
    fn test_codec_default() {
        let codec: CPRemLenCodec = Default::default();
        assert_eq!(codec.value, 0);
        assert_eq!(codec.multiplier, 1);
    }

    #[test]
    fn test_codec_reset() {
        let mut codec: CPRemLenCodec = Default::default();
        codec.value = 10;
        codec.multiplier = 20;
        codec.reset();
        assert_eq!(codec.value, 0);
        assert_eq!(codec.multiplier, 1);
    }

    #[test]
    fn test_codec_encode() {
        let mut codec: CPRemLenCodec = Default::default();

        {
            let remaining_length = CPRemLen(1);
            let mut buf = BytesMut::new();
            assert!(codec.encode(&remaining_length, &mut buf).is_ok());
            assert_eq!(buf.to_vec(), vec![1]);
            codec.reset();
        }

        {
            let remaining_length = CPRemLen(127);
            let mut buf = BytesMut::new();
            assert!(codec.encode(&remaining_length, &mut buf).is_ok());
            assert_eq!(buf.to_vec(), vec![127]);
            codec.reset();
        }

        {
            let remaining_length = CPRemLen(128);
            let mut buf = BytesMut::new();
            assert!(codec.encode(&remaining_length, &mut buf).is_ok());
            assert_eq!(buf.to_vec(), vec![0x80, 0x01]);
            codec.reset();
        }

        {
            let remaining_length = CPRemLen(16_383);
            let mut buf = BytesMut::new();
            assert!(codec.encode(&remaining_length, &mut buf).is_ok());
            assert_eq!(buf.to_vec(), vec![0xFF, 0x7F]);
            codec.reset();
        }

        {
            let remaining_length = CPRemLen(16_384);
            let mut buf = BytesMut::new();
            assert!(codec.encode(&remaining_length, &mut buf).is_ok());
            assert_eq!(buf.to_vec(), vec![0x80, 0x80, 0x01]);
            codec.reset();
        }

        {
            let remaining_length = CPRemLen(2_097_151);
            let mut buf = BytesMut::new();
            assert!(codec.encode(&remaining_length, &mut buf).is_ok());
            assert_eq!(buf.to_vec(), vec![0xFF, 0xFF, 0x7F]);
            codec.reset();
        }

        {
            let remaining_length = CPRemLen(2_097_152);
            let mut buf = BytesMut::new();
            assert!(codec.encode(&remaining_length, &mut buf).is_ok());
            assert_eq!(buf.to_vec(), vec![0x80, 0x80, 0x80, 0x01]);
            codec.reset();
        }

        {
            let remaining_length = CPRemLen(268_435_455);
            let mut buf = BytesMut::new();
            assert!(codec.encode(&remaining_length, &mut buf).is_ok());
            assert_eq!(buf.to_vec(), vec![0xFF, 0xFF, 0xFF, 0x7F]);
            codec.reset();
        }
    }

    #[test]
    fn test_codec_decode() {
        let mut codec: CPRemLenCodec = Default::default();

        {
            let mut buf = BytesMut::from(vec![0x00].as_slice());
            assert_eq!(
                codec
                    .decode(&mut buf)
                    .expect("Ok(Some(0)) is expected")
                    .unwrap(),
                CPRemLen(0)
            );
            codec.reset();
        }

        {
            let mut buf = BytesMut::from(vec![0x7F].as_slice());
            assert_eq!(
                codec
                    .decode(&mut buf)
                    .expect("Ok(Some(127)) is expected")
                    .unwrap(),
                CPRemLen(127)
            );
            codec.reset();
        }

        {
            let mut buf = BytesMut::from(vec![0x80, 0x01].as_slice());

            // emulate loop turn
            codec.decode(&mut buf).expect("Ok is expected");

            assert_eq!(
                codec
                    .decode(&mut buf)
                    .expect("Ok(Some(128)) is expected")
                    .unwrap(),
                CPRemLen(128)
            );
            codec.reset();
        }

        {
            let mut buf = BytesMut::from(vec![0xFF, 0x7F].as_slice());

            // emulate loop turn
            codec.decode(&mut buf).expect("Ok is expected");

            assert_eq!(
                codec
                    .decode(&mut buf)
                    .expect("Ok(Some(16 383)) is expected")
                    .unwrap(),
                CPRemLen(16_383)
            );
            codec.reset();
        }

        {
            let mut buf = BytesMut::from(vec![0x80, 0x80, 0x01].as_slice());

            // emulate loop turn
            codec.decode(&mut buf).expect("Ok is expected");
            codec.decode(&mut buf).expect("Ok is expected");

            assert_eq!(
                codec
                    .decode(&mut buf)
                    .expect("Ok(Some(16 383)) is expected")
                    .unwrap(),
                CPRemLen(16_384)
            );
            codec.reset();
        }

        {
            let mut buf = BytesMut::from(vec![0xFF, 0xFF, 0x7F].as_slice());

            // emulate loop turn
            codec.decode(&mut buf).expect("Ok is expected");
            codec.decode(&mut buf).expect("Ok is expected");

            assert_eq!(
                codec
                    .decode(&mut buf)
                    .expect("Ok(Some(2 097 151)) is expected")
                    .unwrap(),
                CPRemLen(2_097_151)
            );
            codec.reset();
        }

        {
            let mut buf = BytesMut::from(vec![0x80, 0x80, 0x80, 0x01].as_slice());

            // emulate loop turn
            codec.decode(&mut buf).expect("Ok is expected");
            codec.decode(&mut buf).expect("Ok is expected");
            codec.decode(&mut buf).expect("Ok is expected");

            assert_eq!(
                codec
                    .decode(&mut buf)
                    .expect("Ok(Some(2 097 152)) is expected")
                    .unwrap(),
                CPRemLen(2_097_152)
            );
            codec.reset();
        }

        {
            let mut buf = BytesMut::from(vec![0xFF, 0xFF, 0xFF, 0x7F].as_slice());

            // emulate loop turn
            codec.decode(&mut buf).expect("Ok is expected");
            codec.decode(&mut buf).expect("Ok is expected");
            codec.decode(&mut buf).expect("Ok is expected");

            assert_eq!(
                codec
                    .decode(&mut buf)
                    .expect("Ok(Some(268 435 455)) is expected")
                    .unwrap(),
                CPRemLen(268_435_455)
            );
            codec.reset();
        }

        {
            let mut buf = BytesMut::from(vec![0xFF, 0xFF, 0xFF, 0x7F + 1].as_slice());

            // emulate loop turn
            codec.decode(&mut buf).expect("Ok is expected");
            codec.decode(&mut buf).expect("Ok is expected");
            codec.decode(&mut buf).expect("Ok is expected");

            assert!(codec.decode(&mut buf).is_err());
            codec.reset();
        }
    }
}
