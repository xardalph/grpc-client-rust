use prost::Message;

use prost_reflect::{DescriptorPool, DynamicMessage};
use tonic::Status;
use tonic::codec::{Codec, DecodeBuf, Decoder, EncodeBuf};

/// A tonic Codec that can dynamically encode/decode gRPC messages using reflection.
#[derive(Clone)]
pub struct DynamicCodec {
    /// The descriptor pool containing all known protobuf types.
    pub pool: DescriptorPool,
    /// The fully-qualified name of the message type to decode.
    pub message_name: String,
}

impl Codec for DynamicCodec {
    type Encode = DynamicMessage;
    type Decode = DynamicMessage;
    type Encoder = DynamicEncoder;
    type Decoder = DynamicDecoder;

    fn encoder(&mut self) -> Self::Encoder {
        DynamicEncoder
    }

    fn decoder(&mut self) -> Self::Decoder {
        DynamicDecoder {
            pool: self.pool.clone(),
            message_name: self.message_name.clone(),
        }
    }
}

/// Encoder for DynamicMessage
#[derive(Clone)]
pub struct DynamicEncoder;

impl tonic::codec::Encoder for DynamicEncoder {
    type Item = DynamicMessage;
    type Error = Status;

    fn encode(&mut self, item: Self::Item, buf: &mut EncodeBuf<'_>) -> Result<(), Self::Error> {
        // Use prost::Message::encode directly into the provided buffer
        item.encode(buf)
            .map_err(|e| Status::internal(format!("encode error: {e}")))
    }
}

/// Decoder for DynamicMessage
#[derive(Clone)]
pub struct DynamicDecoder {
    pub pool: DescriptorPool,
    pub message_name: String,
}

impl Decoder for DynamicDecoder {
    type Item = DynamicMessage;
    type Error = Status;

    fn decode(&mut self, buf: &mut DecodeBuf<'_>) -> Result<Option<Self::Item>, Self::Error> {
        use bytes::Buf;

        // If there’s no data, we can’t decode yet.
        if !buf.has_remaining() {
            return Ok(None);
        }

        let desc = self
            .pool
            .get_message_by_name(&self.message_name)
            .ok_or_else(|| {
                Status::internal(format!(
                    "message '{}' not found in pool",
                    &self.message_name
                ))
            })?;

        let bytes = buf.copy_to_bytes(buf.remaining());
        let msg = DynamicMessage::decode(desc, bytes)
            .map_err(|e| Status::internal(format!("decode error: {e}")))?;

        Ok(Some(msg))
    }
}
