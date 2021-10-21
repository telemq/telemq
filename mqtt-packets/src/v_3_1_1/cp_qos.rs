use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

/// These two bits specify the QoS level to be used when publishing the Will Message.
///
/// If the Will Flag is set to 0, then the Will QoS MUST be set to 0 (0x00).
///
/// If the Will Flag is set to 1, the value of Will QoS can be 0 (0x00), 1 (0x01),
/// or 2 (0x02). It MUST NOT be 3 (0x03).
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum QoS {
    Zero,
    One,
    Two,
}

impl QoS {
    /// It returns flag bits (4-5) related to a `QoS`;
    pub fn bits(&self) -> u8 {
        match *self {
            QoS::Zero => 0,
            QoS::One => 1,
            QoS::Two => 2,
        }
    }

    pub fn try_from(v: u8) -> std::io::Result<QoS> {
        let qos = match v {
            0 => QoS::Zero,
            1 => QoS::One,
            2 => QoS::Two,
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "QoS codec: malformed QoS byte",
                ));
            }
        };

        Ok(qos)
    }
}

impl PartialOrd for QoS {
    fn partial_cmp(&self, other: &QoS) -> Option<Ordering> {
        Some(self.bits().cmp(&other.bits()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qos_bits() {
        assert_eq!(QoS::Zero.bits(), 0);
        assert_eq!(QoS::One.bits(), 1);
        assert_eq!(QoS::Two.bits(), 2);
    }

    #[test]
    fn test_from_u8() {
        assert_eq!(
            QoS::try_from(0).expect("should decode 0 without errors"),
            QoS::Zero
        );
        assert_eq!(
            QoS::try_from(1).expect("should decode 1 without errors"),
            QoS::One
        );
        assert_eq!(
            QoS::try_from(2).expect("should decode 1 without errors"),
            QoS::Two
        );
        assert_eq!(
            QoS::try_from(5).is_err(),
            true,
            "should return an error for malformed bytes"
        );
    }

    #[test]
    fn test_partial_cmp() {
        let zero = QoS::Zero;
        let one = QoS::One;
        let two = QoS::Two;

        assert!(zero == zero);
        assert!(zero >= zero);
        assert!(zero <= zero);
        assert!(zero < one);
        assert!(zero <= one);
        assert!(zero < two);
        assert!(zero <= two);
        assert!(one == one);
        assert!(one <= one);
        assert!(one >= one);
        assert!(one < two);
        assert!(one <= two);
        assert!(two == two);
        assert!(two >= two);
        assert!(two <= two);
    }
}
