use super::raw_bytes::{ProstField, RawBytes};
use crate::types::message::{Message, MessageType};
use crate::types::protos::KeyValue;

#[derive(Clone, PartialEq, Debug, Default)]
pub struct Request<Payload: ProstField + Default = RawBytes> {
    pub service: String,
    pub method: String,
    pub payload: Payload,
    pub timeout_nano: i64,
    pub metadata: Vec<KeyValue>,
}

impl<Payload: ProstField + Default> Message for Request<Payload> {
    const TYPE_ID: MessageType = MessageType::Request;
}

/*
This is a modified version of the code generated by cargo expand on:
```
#[derive(Clone, PartialEq, prost::Message)]
pub struct Request<Payload: ProstMessage + Default> {
    #[prost(string)]
    pub service: String,

    #[prost(string)]
    pub method: String,

    #[prost(message, required)]
    pub payload: Payload,

    #[prost(int64)]
    pub timeout_nano: i64,

    #[prost(message, repeated)]
    pub metadata: Vec<KeyValue>,
}
```
*/

#[allow(clippy::all)]
impl<Payload: ProstField + Default> ::prost::Message for Request<Payload> {
    #[allow(unused_variables)]
    fn encode_raw(&self, buf: &mut impl ::prost::bytes::BufMut) {
        if self.service != "" {
            ::prost::encoding::string::encode(1u32, &self.service, buf);
        }
        if self.method != "" {
            ::prost::encoding::string::encode(2u32, &self.method, buf);
        }
        self.payload.encode(3u32, buf);
        if self.timeout_nano != 0i64 {
            ::prost::encoding::int64::encode(4u32, &self.timeout_nano, buf);
        }
        for msg in &self.metadata {
            ::prost::encoding::message::encode(5u32, msg, buf);
        }
    }
    #[allow(unused_variables)]
    fn merge_field(
        &mut self,
        tag: u32,
        wire_type: ::prost::encoding::WireType,
        buf: &mut impl ::prost::bytes::Buf,
        ctx: ::prost::encoding::DecodeContext,
    ) -> ::core::result::Result<(), ::prost::DecodeError> {
        const STRUCT_NAME: &'static str = "Request";
        match tag {
            1u32 => {
                let value = &mut self.service;
                ::prost::encoding::string::merge(wire_type, value, buf, ctx).map_err(|mut error| {
                    error.push(STRUCT_NAME, "service");
                    error
                })
            }
            2u32 => {
                let value = &mut self.method;
                ::prost::encoding::string::merge(wire_type, value, buf, ctx).map_err(|mut error| {
                    error.push(STRUCT_NAME, "method");
                    error
                })
            }
            3u32 => {
                let value = &mut self.payload;
                value.merge(wire_type, buf, ctx).map_err(|mut error| {
                    error.push(STRUCT_NAME, "payload");
                    error
                })
            }
            4u32 => {
                let value = &mut self.timeout_nano;
                ::prost::encoding::int64::merge(wire_type, value, buf, ctx).map_err(|mut error| {
                    error.push(STRUCT_NAME, "timeout_nano");
                    error
                })
            }
            5u32 => {
                let value = &mut self.metadata;
                ::prost::encoding::message::merge_repeated(wire_type, value, buf, ctx).map_err(
                    |mut error| {
                        error.push(STRUCT_NAME, "metadata");
                        error
                    },
                )
            }
            _ => ::prost::encoding::skip_field(wire_type, tag, buf, ctx),
        }
    }
    #[inline]
    #[allow(clippy::if_not_else)]
    fn encoded_len(&self) -> usize {
        0 + if self.service != "" {
            ::prost::encoding::string::encoded_len(1u32, &self.service)
        } else {
            0
        } + if self.method != "" {
            ::prost::encoding::string::encoded_len(2u32, &self.method)
        } else {
            0
        } + self.payload.encoded_len(3u32)
            + if self.timeout_nano != 0i64 {
                ::prost::encoding::int64::encoded_len(4u32, &self.timeout_nano)
            } else {
                0
            }
            + ::prost::encoding::message::encoded_len_repeated(5u32, &self.metadata)
    }
    fn clear(&mut self) {
        self.service.clear();
        self.method.clear();
        self.payload.clear();
        self.timeout_nano = 0i64;
        self.metadata.clear();
    }
}
