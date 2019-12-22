
use std::marker::PhantomData;

use crate::{Decoder, Encoder};
use bytes::{Buf, BytesMut, BufMut};

use serde::{Serialize, Deserialize};
use serde_json;

/// A codec for JSON encoding and decoding using serde_json
/// Enc is the type to encode, Dec is the type to decode
/// ```
/// # use futures::{executor, SinkExt, TryStreamExt};
/// # use futures::io::Cursor;
/// use serde::{Serialize, Deserialize};
/// use futures_codec::{JsonCodec, Framed};
/// 
/// #[derive(Serialize, Deserialize)]
/// struct Something {
///     pub data: u16,
/// }
///
/// async move {
///     # let mut buf = vec![];
///     # let stream = Cursor::new(&mut buf);
///     // let stream = ...
///     let codec = JsonCodec::<Something, Something>::new();
///     let mut framed = Framed::new(stream, codec);
///
///     while let Some(s) = framed.try_next().await.unwrap() {
///         println!("{:?}", s.data);
///     }
/// };
/// ```
#[derive(Debug, PartialEq)]
pub struct JsonCodec<Enc, Dec> 
{
    enc: PhantomData<Enc>,
    dec: PhantomData<Dec>,
}

/// JSON Codec error enumeration
#[derive(Debug)]
pub enum JsonCodecError {
    /// IO error
    Io(std::io::Error),
    /// JSON error
    Json(serde_json::Error),
}

impl From<std::io::Error> for JsonCodecError {
    fn from(e: std::io::Error) -> JsonCodecError {
        return JsonCodecError::Io(e);
    }
}

impl From<serde_json::Error> for JsonCodecError {
    fn from(e: serde_json::Error) -> JsonCodecError {
        return JsonCodecError::Json(e);
    }
}

impl <Enc, Dec>JsonCodec<Enc, Dec> 
where 
    for<'de> Dec: Deserialize<'de> + 'static,
    for<'de> Enc: Serialize + 'static,
{
    /// Creates a new `JsonCodec` with the associated types
    pub fn new() -> JsonCodec<Enc, Dec> { 
        JsonCodec {enc: PhantomData, dec: PhantomData}  
    }
}

impl <Enc, Dec>Clone for JsonCodec<Enc, Dec> 
where 
    for<'de> Dec: Deserialize<'de> + 'static,
    for<'de> Enc: Serialize + 'static,
{
    /// Clone creates a new instance of the `JsonCodec`
    fn clone(&self) -> JsonCodec<Enc, Dec> {
        JsonCodec::new()
    }
}

/// Decoder impl parses json objects from bytes
impl <Enc, Dec> Decoder for JsonCodec<Enc, Dec> 
where 
    for<'de> Dec: Deserialize<'de> + 'static,
    for<'de> Enc: Serialize + 'static,
{
    type Item = Dec;
    type Error = JsonCodecError;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {

        // Build streaming JSON iterator over data
        let de = serde_json::Deserializer::from_slice(&buf);
        let mut iter = de.into_iter::<Dec>();

        // Attempt to fetch an item and generate response
        let res = match iter.next() {
            Some(Ok(v)) => Ok(Some(v)),
            Some(Err(ref e)) if e.is_eof() => Ok(None),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        };

        // Update offset from iterator
        let offset = iter.byte_offset();

        // Advance buffer
        buf.advance(offset);

        res
    }
}


/// Encoder impl encodes object streams to bytes
impl <Enc, Dec>Encoder for JsonCodec<Enc, Dec> 
where 
    for<'de> Dec: Deserialize<'de> + 'static,
    for<'de> Enc: Serialize + 'static,
{
    type Item = Enc;
    type Error = JsonCodecError;

    fn encode(&mut self, data: Self::Item, buf: &mut BytesMut) -> Result<(), Self::Error> {
        // Encode json
        let j = serde_json::to_string(&data)?;
        
        // Write to buffer
        buf.reserve(j.len());
        buf.put_slice(&j.as_bytes());

        Ok(())
    }
}


#[cfg(test)]
mod test {
    use bytes::BytesMut;
    use serde::{Serialize, Deserialize};

    use crate::{Decoder, Encoder};
    use super::{JsonCodec};

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct TestStruct {
        pub name: String,
        pub data: u16,
    }

    #[test]
    fn json_codec_encode_decode() {
        let mut codec = JsonCodec::<TestStruct, TestStruct>::new();
        let mut buff = BytesMut::new();

        let item1 = TestStruct{name: "Test name".to_owned(), data: 16};
        codec.encode(item1.clone(), &mut buff).unwrap();

        let item2 = codec.decode(&mut buff).unwrap().unwrap();
        assert_eq!(item1, item2);

        assert_eq!(codec.decode(&mut buff).unwrap(), None);

        assert_eq!(buff.len(), 0);
    }

    #[test]
    fn json_codec_partial_decode() {
        let mut codec = JsonCodec::<TestStruct, TestStruct>::new();
        let mut buff = BytesMut::new();

        let item1 = TestStruct{name: "Test name".to_owned(), data: 34};
        codec.encode(item1.clone(), &mut buff).unwrap();

        let mut start = buff.clone().split_to(4);
        assert_eq!(codec.decode(&mut start).unwrap(), None);

        codec.decode(&mut buff).unwrap().unwrap();

        assert_eq!(buff.len(), 0);
    }
}