use serde::de;

pub(super) struct BytesVisitor;

impl<'de> de::Visitor<'de> for BytesVisitor {
    type Value = Vec<u8>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "expect bytes/bytes<M>")
    }

    fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(v)
    }
}
