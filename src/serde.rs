use std::{collections::HashMap, fmt, io, marker::PhantomData};

use base64::{engine::general_purpose, Engine};
use lz4::{Decoder, EncoderBuilder};
use serde::{
    de::{Error, MapAccess, Visitor},
    ser::Error as SerError,
    Serialize, Serializer,
};

use crate::M;

// A constant representing the key used to store serialized registers.
const REGISTER_KEY: &str = "registers";

/// Represents a visitor for deserializing compressed register values in HLL structures.
///
/// The visitor pattern in Serde allows for data structures to be deserialized
/// in a customized manner. In this case, the `CompressedRegistersVisitor` is
/// tailored for handling the compressed format of the registers.
pub(crate) struct CompressedRegistersVisitor<T>(PhantomData<T>);

impl<T> CompressedRegistersVisitor<T> {
    /// Create a new compressed register visitor.
    pub(crate) fn new() -> Self {
        Self(PhantomData)
    }
}

impl<'de, T> Visitor<'de> for CompressedRegistersVisitor<T>
where
    T: From<[u8; M]>,
{
    type Value = T;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("base64 encoded lz4 compressed sequence of bytes")
    }

    fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        extract_and_decompress(map, [0; M])
    }
}

/// Extracts and decompresses the serialized registers from the provided map.
///
/// # Arguments
///
/// * `map`: The serialized map containing the compressed registers.
/// * `default_registers`: The default registers to be used if no compressed registers are found.
pub(crate) fn extract_and_decompress<'de, A, T>(
    mut map: A,
    default_registers: [u8; M],
) -> Result<T, <A as MapAccess<'de>>::Error>
where
    A: MapAccess<'de>,
    T: From<[u8; M]>,
{
    let mut registers = String::new();

    while let Some((key, value)) = map.next_entry::<String, String>()? {
        if key == REGISTER_KEY {
            registers = value;
        }
    }

    let compressed = general_purpose::STANDARD
        .decode(&registers)
        .map_err(A::Error::custom)?;
    let mut decoder = Decoder::new(io::Cursor::new(compressed)).map_err(A::Error::custom)?;
    let mut result_registers = default_registers;

    io::copy(&mut decoder, &mut result_registers.as_mut_slice()).map_err(A::Error::custom)?;

    Ok(T::from(result_registers))
}

/// Serializes the provided registers into a compressed format suitable for transmission or storage.
///
/// # Arguments
///
/// * `registers`: The registers to be serialized.
/// * `serializer`: The Serde serializer to use.
pub(crate) fn serialize_registers<S>(registers: &[u8; M], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut compressed = Vec::new();
    let mut enc = EncoderBuilder::new()
        .level(16)
        .build(&mut compressed)
        .map_err(S::Error::custom)?;

    io::copy(&mut io::Cursor::new(registers), &mut enc).map_err(S::Error::custom)?;

    let s = general_purpose::STANDARD.encode(&compressed);
    let mut map = HashMap::new();

    map.insert(REGISTER_KEY, s);
    map.serialize(serializer)
}
