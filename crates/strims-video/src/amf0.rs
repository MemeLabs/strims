// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! AMF0, the value encoding of RTMP commands and data messages.
//!
//! [`Decoder`] reads values borrowed from a message body: numbers,
//! booleans, strings and long strings, objects, null, undefined, ECMA
//! arrays, strict arrays and dates. Objects and ECMA arrays end at the
//! object-end marker; an ECMA array's count is a hint and never sizes an
//! allocation. AMF3 switches, references, XML documents, typed objects and
//! the reserved markers are refused with an error.
//!
//! A message's values are bounded: containers nest at most [`MAX_DEPTH`]
//! deep, one decoder reads at most [`MAX_ITEMS`] values (each container,
//! element and property value counts), and a long string holds at most
//! [`MAX_LONG_STRING`] bytes. Strings must be UTF-8.
//!
//! [`encode`] writes values into a `Vec`, and [`Metadata`] reads the
//! fields of an `onMetaData` object that ingest uses.

/// The deepest nesting of containers a message may hold.
pub const MAX_DEPTH: usize = 8;
/// The most values one decoder reads.
pub const MAX_ITEMS: usize = 512;
/// The longest long string accepted, in bytes.
pub const MAX_LONG_STRING: usize = 64 * 1024;

const NUMBER: u8 = 0x00;
const BOOLEAN: u8 = 0x01;
const STRING: u8 = 0x02;
const OBJECT: u8 = 0x03;
const NULL: u8 = 0x05;
const UNDEFINED: u8 = 0x06;
const REFERENCE: u8 = 0x07;
const ECMA_ARRAY: u8 = 0x08;
const OBJECT_END: u8 = 0x09;
const STRICT_ARRAY: u8 = 0x0a;
const DATE: u8 = 0x0b;
const LONG_STRING: u8 = 0x0c;
const XML_DOCUMENT: u8 = 0x0f;
const TYPED_OBJECT: u8 = 0x10;
const AVMPLUS: u8 = 0x11;

/// An AMF0 value. Strings borrow from the decoded input.
#[derive(Debug, Clone, PartialEq)]
pub enum Value<'a> {
    Number(f64),
    Boolean(bool),
    /// A string or long string; [`encode`] writes a long string when the
    /// text exceeds `u16::MAX` bytes.
    String(&'a str),
    /// Properties in wire order.
    Object(Vec<(&'a str, Value<'a>)>),
    Null,
    Undefined,
    /// Properties in wire order.
    EcmaArray(Vec<(&'a str, Value<'a>)>),
    StrictArray(Vec<Value<'a>>),
    /// Milliseconds since the Unix epoch, and the timezone field (which
    /// the format reserves and senders set to 0).
    Date {
        millis: f64,
        timezone: i16,
    },
}

impl<'a> Value<'a> {
    pub fn as_str(&self) -> Option<&'a str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    /// The properties of an object or ECMA array.
    pub fn properties(&self) -> Option<&[(&'a str, Value<'a>)]> {
        match self {
            Value::Object(p) | Value::EcmaArray(p) => Some(p),
            _ => None,
        }
    }

    /// The first property named `key` of an object or ECMA array.
    pub fn get(&self, key: &str) -> Option<&Value<'a>> {
        self.properties()?
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Amf0Error {
    #[error("AMF0 value is truncated")]
    Truncated,
    /// A marker AMF0 reserves or doesn't define, or an object-end marker
    /// where a value belongs.
    #[error("AMF0 marker {0:#04x} is not valid here")]
    Marker(u8),
    #[error("AMF3 values are not supported")]
    Amf3,
    #[error("AMF0 references are not supported")]
    Reference,
    #[error("AMF0 XML documents are not supported")]
    Xml,
    #[error("AMF0 typed objects are not supported")]
    TypedObject,
    /// An empty property name not followed by the object-end marker.
    #[error("AMF0 object has an empty property name")]
    EmptyKey,
    #[error("AMF0 containers nest deeper than {MAX_DEPTH}")]
    Depth,
    #[error("AMF0 message holds more than {MAX_ITEMS} values")]
    Items,
    #[error("AMF0 long string of {0} bytes exceeds {MAX_LONG_STRING}")]
    LongString(u32),
    #[error("AMF0 string is not UTF-8")]
    Utf8,
    /// An encoded property name longer than `u16::MAX` bytes.
    #[error("AMF0 property name of {0} bytes is too long")]
    KeyTooLong(usize),
    /// An encoded long string or strict array longer than `u32::MAX`.
    #[error("AMF0 value of {0} bytes or elements is too long")]
    TooLong(usize),
}

/// Reads the values of one message in order.
pub struct Decoder<'a> {
    input: &'a [u8],
    pos: usize,
    /// Values read so far, at most [`MAX_ITEMS`].
    items: usize,
}

impl<'a> Decoder<'a> {
    pub fn new(input: &'a [u8]) -> Self {
        Decoder {
            input,
            pos: 0,
            items: 0,
        }
    }

    /// Whether every byte has been read.
    pub fn is_empty(&self) -> bool {
        self.pos >= self.input.len()
    }

    /// The bytes not yet read.
    pub fn remaining(&self) -> &'a [u8] {
        self.input.get(self.pos..).unwrap_or_default()
    }

    /// Reads the next value.
    pub fn decode(&mut self) -> Result<Value<'a>, Amf0Error> {
        self.value(0)
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], Amf0Error> {
        let end = self.pos.checked_add(n).ok_or(Amf0Error::Truncated)?;
        let b = self.input.get(self.pos..end).ok_or(Amf0Error::Truncated)?;
        self.pos = end;
        Ok(b)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], Amf0Error> {
        self.take(N)?.try_into().map_err(|_| Amf0Error::Truncated)
    }

    fn u8(&mut self) -> Result<u8, Amf0Error> {
        Ok(self.array::<1>()?[0])
    }

    fn u16(&mut self) -> Result<u16, Amf0Error> {
        Ok(u16::from_be_bytes(self.array()?))
    }

    fn u32(&mut self) -> Result<u32, Amf0Error> {
        Ok(u32::from_be_bytes(self.array()?))
    }

    fn f64(&mut self) -> Result<f64, Amf0Error> {
        Ok(f64::from_be_bytes(self.array()?))
    }

    fn str(&mut self, len: usize) -> Result<&'a str, Amf0Error> {
        std::str::from_utf8(self.take(len)?).map_err(|_| Amf0Error::Utf8)
    }

    fn value(&mut self, depth: usize) -> Result<Value<'a>, Amf0Error> {
        if self.items >= MAX_ITEMS {
            return Err(Amf0Error::Items);
        }
        self.items += 1;
        let marker = self.u8()?;
        Ok(match marker {
            NUMBER => Value::Number(self.f64()?),
            BOOLEAN => Value::Boolean(self.u8()? != 0),
            STRING => {
                let len = self.u16()?;
                Value::String(self.str(usize::from(len))?)
            }
            LONG_STRING => {
                let len = self.u32()?;
                let n = usize::try_from(len).map_err(|_| Amf0Error::LongString(len))?;
                if n > MAX_LONG_STRING {
                    return Err(Amf0Error::LongString(len));
                }
                Value::String(self.str(n)?)
            }
            OBJECT => Value::Object(self.properties(depth + 1)?),
            ECMA_ARRAY => {
                // The count is advisory; the object-end marker ends the array.
                self.u32()?;
                Value::EcmaArray(self.properties(depth + 1)?)
            }
            STRICT_ARRAY => {
                let count = self.u32()?;
                if depth + 1 > MAX_DEPTH {
                    return Err(Amf0Error::Depth);
                }
                let mut values = Vec::new();
                for _ in 0..count {
                    values.push(self.value(depth + 1)?);
                }
                Value::StrictArray(values)
            }
            NULL => Value::Null,
            UNDEFINED => Value::Undefined,
            DATE => {
                let millis = self.f64()?;
                let timezone = i16::from_be_bytes(self.array()?);
                Value::Date { millis, timezone }
            }
            REFERENCE => return Err(Amf0Error::Reference),
            XML_DOCUMENT => return Err(Amf0Error::Xml),
            TYPED_OBJECT => return Err(Amf0Error::TypedObject),
            AVMPLUS => return Err(Amf0Error::Amf3),
            m => return Err(Amf0Error::Marker(m)),
        })
    }

    /// Reads properties up to and including the object-end marker.
    fn properties(&mut self, depth: usize) -> Result<Vec<(&'a str, Value<'a>)>, Amf0Error> {
        if depth > MAX_DEPTH {
            return Err(Amf0Error::Depth);
        }
        let mut props = Vec::new();
        loop {
            let len = self.u16()?;
            if len == 0 {
                return match self.u8()? {
                    OBJECT_END => Ok(props),
                    _ => Err(Amf0Error::EmptyKey),
                };
            }
            let key = self.str(usize::from(len))?;
            props.push((key, self.value(depth)?));
        }
    }
}

/// Reads every value of a message.
pub fn decode_all(input: &[u8]) -> Result<Vec<Value<'_>>, Amf0Error> {
    let mut d = Decoder::new(input);
    let mut values = Vec::new();
    while !d.is_empty() {
        values.push(d.decode()?);
    }
    Ok(values)
}

/// Appends the encoding of `value` to `out`. On error `out` may hold part
/// of the value.
pub fn encode(value: &Value<'_>, out: &mut Vec<u8>) -> Result<(), Amf0Error> {
    match value {
        Value::Number(n) => {
            out.push(NUMBER);
            out.extend_from_slice(&n.to_be_bytes());
        }
        Value::Boolean(b) => out.extend_from_slice(&[BOOLEAN, u8::from(*b)]),
        Value::String(s) => match u16::try_from(s.len()) {
            Ok(len) => {
                out.push(STRING);
                out.extend_from_slice(&len.to_be_bytes());
                out.extend_from_slice(s.as_bytes());
            }
            Err(_) => {
                let len = u32::try_from(s.len()).map_err(|_| Amf0Error::TooLong(s.len()))?;
                out.push(LONG_STRING);
                out.extend_from_slice(&len.to_be_bytes());
                out.extend_from_slice(s.as_bytes());
            }
        },
        Value::Object(props) => {
            out.push(OBJECT);
            encode_properties(props, out)?;
        }
        Value::Null => out.push(NULL),
        Value::Undefined => out.push(UNDEFINED),
        Value::EcmaArray(props) => {
            let count = u32::try_from(props.len()).map_err(|_| Amf0Error::TooLong(props.len()))?;
            out.push(ECMA_ARRAY);
            out.extend_from_slice(&count.to_be_bytes());
            encode_properties(props, out)?;
        }
        Value::StrictArray(values) => {
            let count =
                u32::try_from(values.len()).map_err(|_| Amf0Error::TooLong(values.len()))?;
            out.push(STRICT_ARRAY);
            out.extend_from_slice(&count.to_be_bytes());
            for v in values {
                encode(v, out)?;
            }
        }
        Value::Date { millis, timezone } => {
            out.push(DATE);
            out.extend_from_slice(&millis.to_be_bytes());
            out.extend_from_slice(&timezone.to_be_bytes());
        }
    }
    Ok(())
}

fn encode_properties(props: &[(&str, Value<'_>)], out: &mut Vec<u8>) -> Result<(), Amf0Error> {
    for (key, value) in props {
        let len = u16::try_from(key.len()).map_err(|_| Amf0Error::KeyTooLong(key.len()))?;
        out.extend_from_slice(&len.to_be_bytes());
        out.extend_from_slice(key.as_bytes());
        encode(value, out)?;
    }
    out.extend_from_slice(&[0, 0, OBJECT_END]);
    Ok(())
}

/// The `onMetaData` fields ingest reads. Each is `None` when absent or of
/// another type.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Metadata {
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub framerate: Option<f64>,
    /// The video and audio bitrates the encoder states, in kbit/s.
    pub videodatarate: Option<f64>,
    pub audiodatarate: Option<f64>,
    /// An FLV codec id, or an Enhanced RTMP FourCC as a number.
    pub videocodecid: Option<f64>,
    /// An FLV sound format, or an Enhanced RTMP FourCC as a number.
    pub audiocodecid: Option<f64>,
    pub audiosamplerate: Option<f64>,
    pub encoder: Option<String>,
}

impl Metadata {
    /// Reads an `onMetaData` object or ECMA array; `None` for another
    /// value.
    pub fn from_value(value: &Value<'_>) -> Option<Metadata> {
        value.properties()?;
        let number = |key| value.get(key).and_then(Value::as_number);
        Some(Metadata {
            width: number("width"),
            height: number("height"),
            framerate: number("framerate"),
            videodatarate: number("videodatarate"),
            audiodatarate: number("audiodatarate"),
            videocodecid: number("videocodecid"),
            audiocodecid: number("audiocodecid"),
            audiosamplerate: number("audiosamplerate"),
            encoder: value
                .get("encoder")
                .and_then(Value::as_str)
                .map(str::to_owned),
        })
    }
}

#[cfg(test)]
#[path = "amf0_test.rs"]
mod tests;
