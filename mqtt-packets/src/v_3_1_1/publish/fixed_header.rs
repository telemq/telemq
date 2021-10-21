//! This module contains helper methods for working with fixed
//! header values specific for Publish control packet.

use crate::v_3_1_1::cp_fixed_header::FixedHeader;
use crate::v_3_1_1::cp_flag::Flag;
use crate::v_3_1_1::{CPType, QoS};

const DUP_MASK: u8 = 0b00001000;
const QOS_MASK: u8 = 0b00000110;
const RETAIN_MASK: u8 = 0b00000001;

/// Function that return `true` fixed header is a part of Publish packet
/// AND 3-rd bit of 1-st byte of fixed header is `1`.
pub fn is_dup(fixed_header: &FixedHeader) -> bool {
    fixed_header.cp_type == CPType::Publish && (fixed_header.flag.bits & DUP_MASK != 0)
}

pub fn set_dup(fixed_header: &mut FixedHeader, is_dup: bool) {
    if !is_dup {
        fixed_header.flag.bits &= 0b11110111;
        return;
    }

    fixed_header.flag.bits |= DUP_MASK;
}

/// Function that returns `Ok(QoS)` for Publish packet and error otherwise.
pub fn get_qos_level(fixed_header: &FixedHeader) -> std::io::Result<QoS> {
    // masked bits rotated right in order to remove a leading zero
    let rotated_bits = (fixed_header.flag.bits & QOS_MASK).rotate_right(1);
    QoS::try_from(rotated_bits)
}

/// Function that returns `Ok(QoS)` for a flag.
pub fn get_qos_level_from_flag(flag: &Flag) -> std::io::Result<QoS> {
    // masked bits rotated right in order to remove a leading zero
    let rotated_bits = (flag.bits & QOS_MASK).rotate_right(1);
    QoS::try_from(rotated_bits)
}

pub fn set_qos_level(fixed_header: &mut FixedHeader, qos: &QoS) {
    let qos_bits = match qos {
        QoS::Zero => 0b00000000,
        QoS::One => 0b00000010,
        QoS::Two => 0b00000100,
    };
    let clear_qos = 0b11111001;

    fixed_header.flag.bits &= clear_qos;
    fixed_header.flag.bits |= qos_bits;
}

/// Function that return `true` fixed header is a part of Publish packet
/// AND 0 bit of 1-st byte of fixed header is `1`.
pub fn is_retained(fixed_header: &FixedHeader) -> bool {
    fixed_header.cp_type == CPType::Publish && (fixed_header.flag.bits & RETAIN_MASK != 0)
}

pub fn set_retained(fixed_header: &mut FixedHeader, is_retained: bool) {
    if !is_retained {
        fixed_header.flag.bits &= 0b11111110;
        return;
    }

    fixed_header.flag.bits |= RETAIN_MASK;
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::v_3_1_1::cp_fixed_header::FixedHeader;
    use crate::v_3_1_1::CPRemLen;
    use crate::v_3_1_1::CPType;
    use crate::v_3_1_1::Flag;

    #[test]
    fn test_is_dup() {
        let non_publish_fixed_header = FixedHeader {
            cp_type: CPType::Connack,
            flag: Flag {
                control_packet: CPType::Connack,
                is_reserved: false,
                bits: DUP_MASK,
            },
            remaining_length: CPRemLen::new(0),
        };
        assert!(
            !is_dup(&non_publish_fixed_header),
            "should return false for non-pubish packages"
        );

        let publish_dup_fixed_header = FixedHeader {
            cp_type: CPType::Publish,
            flag: Flag {
                control_packet: CPType::Publish,
                is_reserved: false,
                bits: DUP_MASK,
            },
            remaining_length: CPRemLen::new(0),
        };
        assert!(
            is_dup(&publish_dup_fixed_header),
            "should return true for a pubish package with DUP flag"
        );

        let publish_non_dup_fixed_header = FixedHeader {
            cp_type: CPType::Publish,
            flag: Flag {
                control_packet: CPType::Publish,
                is_reserved: false,
                bits: 0,
            },
            remaining_length: CPRemLen::new(0),
        };
        assert!(
            !is_dup(&publish_non_dup_fixed_header),
            "should return false for a pubish package without DUP flag"
        );
    }

    #[test]
    fn test_get_qos_level() {
        let all_qoss = vec![
            (0, QoS::Zero),
            (0b00000010, QoS::One),
            (0b00000100, QoS::Two),
        ];

        for qos in all_qoss {
            let publish_fixed_header = FixedHeader {
                cp_type: CPType::Publish,
                flag: Flag {
                    control_packet: CPType::Publish,
                    is_reserved: false,
                    bits: qos.0,
                },
                remaining_length: CPRemLen::new(0),
            };

            assert_eq!(
                get_qos_level(&publish_fixed_header).unwrap(),
                QoS::try_from(qos.1.bits()).unwrap(),
            );
        }
    }

    #[test]
    fn test_is_retained() {
        let non_publish_fixed_header = FixedHeader {
            cp_type: CPType::Connack,
            flag: Flag {
                control_packet: CPType::Connack,
                is_reserved: false,
                bits: 1,
            },
            remaining_length: CPRemLen::new(0),
        };
        assert!(
            !is_retained(&non_publish_fixed_header),
            "should return false for non-pubish packages"
        );

        let publish_retained_fixed_header = FixedHeader {
            cp_type: CPType::Publish,
            flag: Flag {
                control_packet: CPType::Publish,
                is_reserved: false,
                bits: 1,
            },
            remaining_length: CPRemLen::new(0),
        };
        assert!(
            is_retained(&publish_retained_fixed_header),
            "should return true for a pubish retained package"
        );

        let publish_non_retained_fixed_header = FixedHeader {
            cp_type: CPType::Publish,
            flag: Flag {
                control_packet: CPType::Publish,
                is_reserved: false,
                bits: 0,
            },
            remaining_length: CPRemLen::new(0),
        };
        assert!(
            !is_retained(&publish_non_retained_fixed_header),
            "should return false for a pubish non-retained package"
        );
    }
}
