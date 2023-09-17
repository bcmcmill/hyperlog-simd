use std::collections::HashMap;
use std::fmt;
use std::io::{self};

use base64::{engine::general_purpose, Engine};
use lz4::{Decoder, EncoderBuilder};
use serde::de::MapAccess;
use serde::{
    de::{self, Deserializer, Error, Visitor},
    ser::Error as SerError,
    Deserialize, Serialize, Serializer,
};

use crate::M;
use crate::{classic::HyperLogLog, plusplus::HyperLogLogPlusPlus};

struct HyperLogLogVisitor;

impl<'de> Visitor<'de> for HyperLogLogVisitor {
    type Value = HyperLogLog;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("base64 encoded lz4 compressed sequence of bytes")
    }

    fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
    where
        A: de::MapAccess<'de>,
    {
        let hll = extract_and_decompress(map)?;

        Ok(hll)
    }
}

impl<'de> Deserialize<'de> for HyperLogLog {
    fn deserialize<D>(deserializer: D) -> Result<HyperLogLog, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(HyperLogLogVisitor)
    }
}

fn extract_and_decompress<'de, A>(mut map: A) -> Result<HyperLogLog, <A as MapAccess<'de>>::Error>
where
    A: de::MapAccess<'de>,
{
    let mut registers = String::new();
    while let Some((key, value)) = map.next_entry::<String, String>()? {
        if key == "registers" {
            registers = value;
        }
    }
    let compressed = general_purpose::STANDARD
        .decode(&registers)
        .map_err(A::Error::custom)?;
    let mut decoder = Decoder::new(io::Cursor::new(compressed)).map_err(A::Error::custom)?;
    let mut decompressed = vec![];
    io::copy(&mut decoder, &mut decompressed).map_err(A::Error::custom)?;
    let mut hll = HyperLogLog { registers: [0; M] };
    io::copy(
        &mut io::Cursor::new(decompressed),
        &mut hll.registers.as_mut_slice(),
    )
    .map_err(A::Error::custom)?;
    Ok(hll)
}

impl Serialize for HyperLogLog {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut compressed = Vec::new();
        let mut enc = EncoderBuilder::new()
            .level(16)
            .build(&mut compressed)
            .map_err(S::Error::custom)?;

        io::copy(&mut io::Cursor::new(self.registers), &mut enc).map_err(S::Error::custom)?;

        let s = general_purpose::STANDARD.encode(&compressed);
        let mut map = HashMap::new();

        map.insert("registers", s);
        map.serialize(serializer)
    }
}
struct HyperLogLogPlusPlusVisitor;

impl<'de> Visitor<'de> for HyperLogLogPlusPlusVisitor {
    type Value = HyperLogLogPlusPlus;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("base64 encoded lz4 compressed sequence of bytes")
    }

    fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
    where
        A: de::MapAccess<'de>,
    {
        let hll = extract_and_decompress_pp(map)?;

        Ok(hll)
    }
}

impl<'de> Deserialize<'de> for HyperLogLogPlusPlus {
    fn deserialize<D>(deserializer: D) -> Result<HyperLogLogPlusPlus, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(HyperLogLogPlusPlusVisitor)
    }
}

fn extract_and_decompress_pp<'de, A>(
    mut map: A,
) -> Result<HyperLogLogPlusPlus, <A as MapAccess<'de>>::Error>
where
    A: de::MapAccess<'de>,
{
    let mut registers = String::new();
    while let Some((key, value)) = map.next_entry::<String, String>()? {
        if key == "registers" {
            registers = value;
        }
    }
    let compressed = general_purpose::STANDARD
        .decode(&registers)
        .map_err(A::Error::custom)?;
    let mut decoder = Decoder::new(io::Cursor::new(compressed)).map_err(A::Error::custom)?;
    let mut decompressed = vec![];
    io::copy(&mut decoder, &mut decompressed).map_err(A::Error::custom)?;
    let mut hll = HyperLogLogPlusPlus { registers: [0; M] };
    io::copy(
        &mut io::Cursor::new(decompressed),
        &mut hll.registers.as_mut_slice(),
    )
    .map_err(A::Error::custom)?;
    Ok(hll)
}

impl Serialize for HyperLogLogPlusPlus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut compressed = Vec::new();
        let mut enc = EncoderBuilder::new()
            .level(16)
            .build(&mut compressed)
            .map_err(S::Error::custom)?;

        io::copy(&mut io::Cursor::new(self.registers), &mut enc).map_err(S::Error::custom)?;

        let s = general_purpose::STANDARD.encode(&compressed);
        let mut map = HashMap::new();

        map.insert("registers", s);
        map.serialize(serializer)
    }
}
