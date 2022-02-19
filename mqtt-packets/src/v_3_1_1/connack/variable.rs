use bytes::BytesMut;

use super::flags::{Flags, FlagsCodec};
use super::return_code::{ReturnCode, ReturnCodeCodec};

/// Connack specific variable header + payload.
#[derive(Debug, PartialEq, Clone)]
pub struct Variable {
    // variable header
    pub flags: Flags,
    pub return_code: ReturnCode,
}

impl Variable {
    pub fn create_with(flags: Flags, return_code: ReturnCode) -> Self {
        Variable {
            flags: flags,
            return_code: return_code,
        }
    }
}

pub struct VariableCodec;

impl VariableCodec {
    pub fn new() -> Self {
        VariableCodec {}
    }

    pub fn encode(&mut self, item: &Variable, dst: &mut BytesMut) -> Result<(), std::io::Error> {
        let flags = &item.flags;
        let return_code = &item.return_code;
        let mut flags_codec = FlagsCodec::new();
        let mut return_code_codec = ReturnCodeCodec::new();

        flags_codec.encode(flags, dst)?;
        return_code_codec.encode(return_code, dst)
    }

    pub fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Variable>, std::io::Error> {
        let flags = {
            let mut flags_codec = FlagsCodec::new();
            flags_codec.decode(src)?.unwrap()
        };

        let return_code = {
            let mut return_code_codec = ReturnCodeCodec::new();
            return_code_codec.decode(src)?.unwrap()
        };

        Ok(Some(Variable::create_with(flags, return_code)))
    }
}
