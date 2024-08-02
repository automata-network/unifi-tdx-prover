use std::{fmt::{Display, Formatter}, ops::Deref};


use hex::FromHexError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Default, PartialEq, Eq, Clone, Ord, PartialOrd, Hash)]
pub struct HexBytes(Vec<u8>);

impl Display for HexBytes {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "0x{}", hex::encode(&self.0))
    }
}

impl core::fmt::Debug for HexBytes {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "0x{}", hex::encode(&self.0))
    }
}

impl HexBytes {
    pub fn into_vec(self) -> Vec<u8> {
        self.0
    }
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
    pub fn to_utf8(&self) -> std::borrow::Cow<'_, str> {
        String::from_utf8_lossy(&self.0)
    }
}

impl Deref for HexBytes {
    type Target = Vec<u8>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PartialEq<[u8]> for HexBytes {
    fn eq(&self, other: &[u8]) -> bool {
        self.0.eq(other)
    }
}

impl PartialEq<Vec<u8>> for HexBytes {
    fn eq(&self, other: &Vec<u8>) -> bool {
        self.0.eq(other)
    }
}

impl From<&str> for HexBytes {
    fn from(val: &str) -> HexBytes {
        Self(val.into())
    }
}

impl HexBytes {
    pub fn new() -> Self {
        Self(Vec::new())
    }
    pub fn from_hex(data: &[u8]) -> Result<Self, FromHexError> {
        let data = if data.len() > 0 && &data[..2] == b"0x" {
            hex::decode(&data[2..])?
        } else {
            hex::decode(&data)?
        };
        Ok(Self(data))
    }
}

impl From<&ethers_core::types::Bytes> for HexBytes {
    fn from(value: &ethers_core::types::Bytes) -> Self {
        (value.deref()).into()
    }
}

impl From<String> for HexBytes {
    fn from(val: String) -> Self {
        val.into_bytes().into()
    }
}

impl From<Vec<u8>> for HexBytes {
    fn from(val: Vec<u8>) -> HexBytes {
        Self(val.into())
    }
}

impl From<&[u8]> for HexBytes {
    fn from(val: &[u8]) -> HexBytes {
        Self(val.into())
    }
}

impl From<HexBytes> for Vec<u8> {
    fn from(val: HexBytes) -> Vec<u8> {
        val.0
    }
}

impl<'de> Deserialize<'de> for HexBytes {
    fn deserialize<D>(deserializer: D) -> Result<HexBytes, D::Error>
    where
        D: Deserializer<'de>,
    {
        let str = String::deserialize(deserializer)?;
        let str = str.trim_start_matches("0x");
        use serde::de::Error;
        let result = hex::decode(&str).map_err(|e| D::Error::custom(format!("{}", e)))?;
        Ok(result.into())
    }
}

impl Serialize for HexBytes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let val = format!("0x{}", hex::encode(&self.0));
        serializer.serialize_str(&val)
    }
}

impl rlp::Decodable for HexBytes {
    fn decode(rlp: &rlp::Rlp) -> Result<Self, rlp::DecoderError> {
        Ok(HexBytes(<Vec<u8>>::decode(rlp)?))
    }
}
impl rlp::Encodable for HexBytes {
    fn rlp_append(&self, s: &mut rlp::RlpStream) {
        self.0.rlp_append(s)
    }
}