use crate::v_3_1_1::QoS;

#[derive(Clone, Debug, PartialEq)]
pub enum ReturnCode {
  SuccessZero,
  SuccessOne,
  SuccessTwo,
  Failure,
}

impl ReturnCode {
  const ZERO: u8 = 0x00;
  const ONE: u8 = 0x01;
  const TWO: u8 = 0x02;
  const FAILURE: u8 = 0x80;

  pub fn try_from(u: u8) -> std::io::Result<Self> {
    match u {
      Self::ZERO => Ok(ReturnCode::SuccessZero),
      Self::ONE => Ok(ReturnCode::SuccessOne),
      Self::TWO => Ok(ReturnCode::SuccessTwo),
      Self::FAILURE => Ok(ReturnCode::Failure),
      _ => Err(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        format!("Suback payload: return code {} is not acceptable", u),
      )),
    }
  }

  pub fn from_qos(qos: &QoS) -> Self {
    match *qos {
      QoS::Zero => ReturnCode::SuccessZero,
      QoS::One => ReturnCode::SuccessTwo,
      QoS::Two => ReturnCode::SuccessTwo,
    }
  }

  pub fn as_u8(&self) -> u8 {
    match self {
      &ReturnCode::SuccessZero => Self::ZERO,
      &ReturnCode::SuccessOne => Self::ONE,
      &ReturnCode::SuccessTwo => Self::TWO,
      &ReturnCode::Failure => Self::FAILURE,
    }
  }
}
