use bytes::BytesMut;

use crate::v_3_1_1::basic_variable::{
    Variable as BasicVariable, VariableCodec as BasicVariableCodec,
};
use crate::v_3_1_1::connack::variable::{
    Variable as ConnackVariable, VariableCodec as ConnackVariableCodec,
};
use crate::v_3_1_1::connect::variable::{
    Variable as ConnectVariable, VariableCodec as ConnectVariableCodec,
};
use crate::v_3_1_1::cp_flag::Flag;
use crate::v_3_1_1::cp_type::CPType;
use crate::v_3_1_1::publish::{
    fixed_header::get_qos_level_from_flag,
    variable::{Variable as PublishVariable, VariableCodec as PublishVariableCodec},
};
use crate::v_3_1_1::suback::variable::{
    Variable as SubackVariable, VariableCodec as SubackVariableCodec,
};
use crate::v_3_1_1::subscribe::variable::{
    Variable as SubscribeVariable, VariableCodec as SubscribeVariableCodec,
};
use crate::v_3_1_1::unsubscribe::variable::{
    Variable as UnsubscribeVariable, VariableCodec as UnsubscribeVariableCodec,
};

#[derive(Debug, Clone)]
pub enum Variable {
    Connect(ConnectVariable),
    Connack(ConnackVariable),
    Disconnect,
    Pingreq,
    Pingresp,
    Puback(BasicVariable),
    Pubcomp(BasicVariable),
    Publish(PublishVariable),
    Pubrec(BasicVariable),
    Pubrel(BasicVariable),
    Suback(SubackVariable),
    Subscribe(SubscribeVariable),
    Unsuback(BasicVariable),
    Unsubscribe(UnsubscribeVariable),
}

pub enum VariableCodec {
    Connect(ConnectVariableCodec),
    Connack(ConnackVariableCodec),
    Disconnect,
    Pingreq,
    Pingresp,
    Puback(BasicVariableCodec),
    Pubcomp(BasicVariableCodec),
    Publish(PublishVariableCodec),
    Pubrec(BasicVariableCodec),
    Pubrel(BasicVariableCodec),
    Suback(SubackVariableCodec),
    Subscribe(SubscribeVariableCodec),
    Unsuback(BasicVariableCodec),
    Unsubscribe(UnsubscribeVariableCodec),
}

impl VariableCodec {
    pub fn create(flag: &Flag) -> ::std::io::Result<Self> {
        Ok(match flag.control_packet {
            CPType::Connect => VariableCodec::Connect(ConnectVariableCodec::new()),
            CPType::Connack => VariableCodec::Connack(ConnackVariableCodec::new()),
            CPType::Disconnect => VariableCodec::Disconnect,
            CPType::Pingreq => VariableCodec::Pingreq,
            CPType::Pingresp => VariableCodec::Pingresp,
            CPType::Puback => VariableCodec::Puback(BasicVariableCodec::new()),
            CPType::Pubcomp => VariableCodec::Pubcomp(BasicVariableCodec::new()),
            CPType::Publish => {
                VariableCodec::Publish(PublishVariableCodec::new(get_qos_level_from_flag(flag)?))
            }
            CPType::Pubrec => VariableCodec::Pubrec(BasicVariableCodec::new()),
            CPType::Pubrel => VariableCodec::Pubrel(BasicVariableCodec::new()),
            CPType::Suback => VariableCodec::Suback(SubackVariableCodec::new()),
            CPType::Subscribe => VariableCodec::Subscribe(SubscribeVariableCodec::new()),
            CPType::Unsuback => VariableCodec::Unsuback(BasicVariableCodec::new()),
            CPType::Unsubscribe => VariableCodec::Unsubscribe(UnsubscribeVariableCodec::new()),
        })
    }

    pub fn encode(&mut self, item: &Variable, dst: &mut BytesMut) -> Result<(), std::io::Error> {
        match self {
            VariableCodec::Connect(ref mut codec) => match item {
                &Variable::Connect(ref variable) => {
                    let res = codec.encode(variable, dst);
                    res
                }
                _ => {
                    return Err(::std::io::Error::new(
                        ::std::io::ErrorKind::Other,
                        "Mismatched Variable Header and its codec types.",
                    ))
                }
            },
            VariableCodec::Connack(ref mut codec) => match item {
                &Variable::Connack(ref variable) => {
                    let res = codec.encode(variable, dst);
                    res
                }
                _ => {
                    return Err(::std::io::Error::new(
                        ::std::io::ErrorKind::Other,
                        "Mismatched Variable Header and its codec types.",
                    ))
                }
            },
            VariableCodec::Disconnect => match item {
                &Variable::Disconnect => Ok(()),
                _ => {
                    return Err(::std::io::Error::new(
                        ::std::io::ErrorKind::Other,
                        "Mismatched Variable Header and its codec types.",
                    ))
                }
            },
            VariableCodec::Pingreq => match item {
                &Variable::Pingreq => Ok(()),
                _ => {
                    return Err(::std::io::Error::new(
                        ::std::io::ErrorKind::Other,
                        "Mismatched Variable Header and its codec types.",
                    ))
                }
            },
            VariableCodec::Pingresp => match item {
                &Variable::Pingresp => Ok(()),
                _ => {
                    return Err(::std::io::Error::new(
                        ::std::io::ErrorKind::Other,
                        "Mismatched Variable Header and its codec types.",
                    ))
                }
            },
            VariableCodec::Puback(ref mut codec) => match item {
                &Variable::Puback(ref variable) => {
                    let res = codec.encode(variable, dst);
                    res
                }
                _ => {
                    return Err(::std::io::Error::new(
                        ::std::io::ErrorKind::Other,
                        "Mismatched Variable Header and its codec types.",
                    ))
                }
            },
            VariableCodec::Pubcomp(ref mut codec) => match item {
                &Variable::Pubcomp(ref variable) => {
                    let res = codec.encode(variable, dst);
                    res
                }
                _ => {
                    return Err(::std::io::Error::new(
                        ::std::io::ErrorKind::Other,
                        "Mismatched Variable Header and its codec types.",
                    ))
                }
            },
            VariableCodec::Publish(ref mut codec) => match item {
                &Variable::Publish(ref variable) => {
                    let res = codec.encode(variable, dst);
                    res
                }
                _ => {
                    return Err(::std::io::Error::new(
                        ::std::io::ErrorKind::Other,
                        "Mismatched Variable Header and its codec types.",
                    ))
                }
            },
            VariableCodec::Pubrec(ref mut codec) => match item {
                &Variable::Pubrec(ref variable) => {
                    let res = codec.encode(variable, dst);
                    res
                }
                _ => {
                    return Err(::std::io::Error::new(
                        ::std::io::ErrorKind::Other,
                        "Mismatched Variable Header and its codec types.",
                    ))
                }
            },
            VariableCodec::Pubrel(ref mut codec) => match item {
                &Variable::Pubrel(ref variable) => {
                    let res = codec.encode(variable, dst);
                    res
                }
                _ => {
                    return Err(::std::io::Error::new(
                        ::std::io::ErrorKind::Other,
                        "Mismatched Variable Header and its codec types.",
                    ))
                }
            },
            VariableCodec::Suback(ref mut codec) => match item {
                &Variable::Suback(ref variable) => {
                    let res = codec.encode(variable, dst);
                    res
                }
                _ => {
                    return Err(::std::io::Error::new(
                        ::std::io::ErrorKind::Other,
                        "Mismatched Variable Header and its codec types.",
                    ))
                }
            },
            VariableCodec::Subscribe(ref mut codec) => match item {
                &Variable::Subscribe(ref variable) => {
                    let res = codec.encode(variable, dst);
                    res
                }
                _ => {
                    return Err(::std::io::Error::new(
                        ::std::io::ErrorKind::Other,
                        "Mismatched Variable Header and its codec types.",
                    ))
                }
            },
            VariableCodec::Unsuback(ref mut codec) => match item {
                &Variable::Unsuback(ref variable) => {
                    let res = codec.encode(variable, dst);
                    res
                }
                _ => {
                    return Err(::std::io::Error::new(
                        ::std::io::ErrorKind::Other,
                        "Mismatched Variable Header and its codec types.",
                    ))
                }
            },
            VariableCodec::Unsubscribe(ref mut codec) => match item {
                &Variable::Unsubscribe(ref variable) => {
                    let res = codec.encode(variable, dst);
                    res
                }
                _ => {
                    return Err(::std::io::Error::new(
                        ::std::io::ErrorKind::Other,
                        "Mismatched Variable Header and its codec types.",
                    ))
                }
            },
        }
    }

    pub fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Variable>, std::io::Error> {
        match self {
            VariableCodec::Connect(ref mut codec) => {
                deep_map(codec.decode(src), |variable| Variable::Connect(variable))
            }
            VariableCodec::Connack(ref mut codec) => {
                deep_map(codec.decode(src), |variable| Variable::Connack(variable))
            }
            VariableCodec::Disconnect => Ok(Some(Variable::Disconnect)),
            VariableCodec::Pingreq => Ok(Some(Variable::Pingreq)),
            VariableCodec::Pingresp => Ok(Some(Variable::Pingresp)),
            VariableCodec::Puback(ref mut codec) => {
                deep_map(codec.decode(src), |variable| Variable::Puback(variable))
            }
            VariableCodec::Pubcomp(ref mut codec) => {
                deep_map(codec.decode(src), |variable| Variable::Pubcomp(variable))
            }
            VariableCodec::Publish(ref mut codec) => {
                deep_map(codec.decode(src), |variable| Variable::Publish(variable))
            }
            VariableCodec::Pubrec(ref mut codec) => {
                deep_map(codec.decode(src), |variable| Variable::Pubrec(variable))
            }
            VariableCodec::Pubrel(ref mut codec) => {
                deep_map(codec.decode(src), |variable| Variable::Pubrel(variable))
            }
            VariableCodec::Suback(ref mut codec) => {
                deep_map(codec.decode(src), |variable| Variable::Suback(variable))
            }
            VariableCodec::Subscribe(ref mut codec) => {
                deep_map(codec.decode(src), |variable| Variable::Subscribe(variable))
            }
            VariableCodec::Unsuback(ref mut codec) => {
                deep_map(codec.decode(src), |variable| Variable::Unsuback(variable))
            }
            VariableCodec::Unsubscribe(ref mut codec) => deep_map(codec.decode(src), |variable| {
                Variable::Unsubscribe(variable)
            }),
        }
    }
}

type ResOpt<T, E> = Result<Option<T>, E>;

fn deep_map<I, T, F, E>(v: ResOpt<I, E>, f: F) -> ResOpt<T, E>
where
    F: FnOnce(I) -> T,
{
    match v {
        Ok(Some(i)) => Ok(Some(f(i))),
        Ok(None) => Ok(None),
        Err(err) => Err(err),
    }
}
