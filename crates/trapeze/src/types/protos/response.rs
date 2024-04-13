use super::raw_bytes::{ProstField, RawBytes};
use crate::types::message::{Message, MessageType};
use crate::types::protos::Status;

#[derive(Clone, PartialEq, Debug, Default)]
pub struct Response<Payload: ProstField + Default = RawBytes> {
    pub status: Option<Status>,
    pub payload: Payload,
}

impl<Payload: ProstField + Default> Message for Response<Payload> {
    const TYPE_ID: MessageType = MessageType::Response;
}

impl Response<()> {
    pub fn error(status: Status) -> Self {
        Self {
            status: Some(status),
            payload: (),
        }
    }
}

impl<Payload: ProstField + Default> Response<Payload> {
    pub fn ok(payload: Payload) -> Self {
        let status = None;
        Self { status, payload }
    }
}

/*
This is a modified version of the code generated by cargo expand on:
````
#[derive(Clone, PartialEq, prost::Message)]
pub struct Response<Payload: ProstMessage + Default> {
    #[prost(message)]
    pub status: Option<Status>,

    #[prost(message, required)]
    pub payload: Payload,
}
```
*/

#[allow(clippy::all)]
impl<Payload: ProstField + Default> ::prost::Message for Response<Payload> {
    #[allow(unused_variables)]
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: ::prost::bytes::BufMut,
    {
        if let Some(ref msg) = self.status {
            ::prost::encoding::message::encode(1u32, msg, buf);
        }
        self.payload.encode(2u32, buf);
    }
    #[allow(unused_variables)]
    fn merge_field<B>(
        &mut self,
        tag: u32,
        wire_type: ::prost::encoding::WireType,
        buf: &mut B,
        ctx: ::prost::encoding::DecodeContext,
    ) -> ::core::result::Result<(), ::prost::DecodeError>
    where
        B: ::prost::bytes::Buf,
    {
        const STRUCT_NAME: &'static str = "Response";
        match tag {
            1u32 => {
                let value = &mut self.status;
                ::prost::encoding::message::merge(
                    wire_type,
                    value.get_or_insert_with(::core::default::Default::default),
                    buf,
                    ctx,
                )
                .map_err(|mut error| {
                    error.push(STRUCT_NAME, "status");
                    error
                })
            }
            2u32 => {
                let value = &mut self.payload;
                value.merge(wire_type, buf, ctx).map_err(|mut error| {
                    error.push(STRUCT_NAME, "payload");
                    error
                })
            }
            _ => ::prost::encoding::skip_field(wire_type, tag, buf, ctx),
        }
    }
    #[inline]
    fn encoded_len(&self) -> usize {
        0 + self
            .status
            .as_ref()
            .map_or(0, |msg| ::prost::encoding::message::encoded_len(1u32, msg))
            + self.payload.encoded_len(2u32)
    }
    fn clear(&mut self) {
        self.status = ::core::option::Option::None;
        self.payload.clear();
    }
}
