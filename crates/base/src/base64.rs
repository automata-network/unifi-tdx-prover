use alloy::primitives::Bytes;

#[derive(Debug)]
pub struct Base64Bytes(pub Bytes);

impl std::ops::Deref for Base64Bytes {
    type Target = Bytes;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl serde::Serialize for Base64Bytes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        Base64Bytes::serialize(&self.0, serializer)
    }
}

impl<'de> serde::Deserialize<'de> for Base64Bytes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self(Base64Bytes::deserialize(deserializer)?))
    }
}

impl Base64Bytes {
    pub fn serialize<S, Input>(
        bytes: Input,
        serializer: S,
    ) -> ::std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
        Input: AsRef<[u8]>,
    {
        use base64::engine::general_purpose::STANDARD;
        use base64::Engine as _;
        serializer.serialize_str(&STANDARD.encode(bytes.as_ref()))
    }

    pub fn deserialize<'de, D, Output>(deserializer: D) -> ::std::result::Result<Output, D::Error>
    where
        D: serde::Deserializer<'de>,
        Output: From<Vec<u8>>,
    {
        struct Base64Visitor;

        impl<'de> serde::de::Visitor<'de> for Base64Visitor {
            type Value = Vec<u8>;

            fn expecting(&self, formatter: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                write!(formatter, "base64 ASCII text")
            }

            fn visit_str<E>(self, v: &str) -> ::std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                use base64::engine::general_purpose::STANDARD;
                use base64::Engine as _;
                STANDARD.decode(v).map_err(serde::de::Error::custom)
            }
        }

        deserializer
            .deserialize_str(Base64Visitor)
            .map(|vec| Output::from(vec))
    }
}
