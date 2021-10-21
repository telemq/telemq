use bytes::BytesMut;

use super::return_code::ReturnCode;
use crate::v_3_1_1::PacketId;

#[derive(Debug, PartialEq, Clone)]
pub struct Variable {
  /// The Packet Identifier field.
  pub packet_id: PacketId,
  pub return_codes: Vec<ReturnCode>,
}

impl Variable {
  pub const PACKET_ID_LEN: usize = 2;
}

pub struct VariableCodec;

impl VariableCodec {
  pub const PACKET_ID_LEN: usize = Variable::PACKET_ID_LEN;

  /// Factory method that creates new instance of `VariableHeaderCodec`.
  pub fn new() -> Self {
    VariableCodec {}
  }

  pub fn encode(&mut self, item: &Variable, dst: &mut BytesMut) -> Result<(), std::io::Error> {
    dst.extend_from_slice(item.packet_id.as_slice());

    {
      let mut bytes: Vec<u8> = Vec::with_capacity(item.return_codes.len());

      for rc in &item.return_codes {
        bytes.push(rc.as_u8());
      }

      dst.extend(bytes);
    }

    Ok(())
  }

  pub fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Variable>, std::io::Error> {
    let packet_id = src.split_to(Self::PACKET_ID_LEN).to_vec();
    let bytes = src.to_vec();
    let mut return_codes = Vec::with_capacity(src.len());

    for b in bytes {
      return_codes.push(ReturnCode::try_from(b)?);
    }

    Ok(Some(Variable {
      packet_id,
      return_codes,
    }))
  }
}
