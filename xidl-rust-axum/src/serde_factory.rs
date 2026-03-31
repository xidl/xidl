use crate::{Error, Result};
use serde::Serialize;
use serde::de::DeserializeOwned;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SerdeKind {
    Json,
    Form,
    #[cfg(feature = "msgpack")]
    Msgpack,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SerializeFactory {
    mime: &'static str,
    kind: SerdeKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeserializeFactory {
    mime: &'static str,
    kind: SerdeKind,
}

impl SerializeFactory {
    pub const fn new(mime: &'static str) -> Self {
        Self {
            mime,
            kind: supported_kind_from_mime(mime),
        }
    }

    pub const fn mime(&self) -> &'static str {
        self.mime
    }

    pub fn to_vec<T>(&self, value: &T) -> Result<Vec<u8>>
    where
        T: Serialize,
    {
        match self.kind {
            SerdeKind::Json => serde_json::to_vec(value)
                .map_err(|err| Error::new(500, format!("serialize {} failed: {err}", self.mime))),
            SerdeKind::Form => serde_urlencoded::to_string(value)
                .map(|value| value.into_bytes())
                .map_err(|err| Error::new(500, format!("serialize {} failed: {err}", self.mime))),
            #[cfg(feature = "msgpack")]
            SerdeKind::Msgpack => rmp_serde::to_vec(value)
                .map_err(|err| Error::new(500, format!("serialize {} failed: {err}", self.mime))),
        }
    }
}

impl DeserializeFactory {
    pub const fn new(mime: &'static str) -> Self {
        Self {
            mime,
            kind: supported_kind_from_mime(mime),
        }
    }

    pub const fn mime(&self) -> &'static str {
        self.mime
    }

    pub fn from_slice<T>(&self, bytes: &[u8]) -> Result<T>
    where
        T: DeserializeOwned,
    {
        match self.kind {
            SerdeKind::Json => serde_json::from_slice(bytes)
                .map_err(|err| Error::new(400, format!("deserialize {} failed: {err}", self.mime))),
            SerdeKind::Form => serde_urlencoded::from_bytes(bytes)
                .map_err(|err| Error::new(400, format!("deserialize {} failed: {err}", self.mime))),
            #[cfg(feature = "msgpack")]
            SerdeKind::Msgpack => rmp_serde::from_slice(bytes)
                .map_err(|err| Error::new(400, format!("deserialize {} failed: {err}", self.mime))),
        }
    }
}

const fn supported_kind_from_mime(mime: &str) -> SerdeKind {
    if eq_ignore_ascii_case(mime, "application/json") {
        SerdeKind::Json
    } else if eq_ignore_ascii_case(mime, "application/x-www-form-urlencoded") {
        SerdeKind::Form
    } else if cfg!(feature = "msgpack") && eq_ignore_ascii_case(mime, "application/msgpack") {
        #[cfg(feature = "msgpack")]
        {
            SerdeKind::Msgpack
        }
        #[cfg(not(feature = "msgpack"))]
        {
            panic!("unsupported mime type")
        }
    } else if eq_ignore_ascii_case(mime, "application/msgpack") {
        panic!("unsupported mime type")
    } else {
        panic!("unsupported mime type")
    }
}

const fn eq_ignore_ascii_case(lhs: &str, rhs: &str) -> bool {
    let lhs = lhs.as_bytes();
    let rhs = rhs.as_bytes();
    if lhs.len() != rhs.len() {
        return false;
    }
    let mut index = 0;
    while index < lhs.len() {
        if to_ascii_lower(lhs[index]) != to_ascii_lower(rhs[index]) {
            return false;
        }
        index += 1;
    }
    true
}

const fn to_ascii_lower(byte: u8) -> u8 {
    if byte >= b'A' && byte <= b'Z' {
        byte + 32
    } else {
        byte
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    struct Payload {
        name: String,
        age: u32,
    }

    #[test]
    fn json_factory_round_trip() {
        let payload = Payload {
            name: "alice".to_string(),
            age: 3,
        };
        let bytes = SerializeFactory::new("application/json")
            .to_vec(&payload)
            .unwrap();
        let decoded: Payload = DeserializeFactory::new("application/json")
            .from_slice(&bytes)
            .unwrap();
        assert_eq!(decoded, payload);
    }

    #[test]
    fn form_factory_round_trip() {
        let payload = Payload {
            name: "alice".to_string(),
            age: 3,
        };
        let bytes = SerializeFactory::new("application/x-www-form-urlencoded")
            .to_vec(&payload)
            .unwrap();
        let decoded: Payload = DeserializeFactory::new("application/x-www-form-urlencoded")
            .from_slice(&bytes)
            .unwrap();
        assert_eq!(decoded, payload);
    }

    #[cfg(feature = "msgpack")]
    #[test]
    fn msgpack_factory_round_trip() {
        let payload = Payload {
            name: "alice".to_string(),
            age: 3,
        };
        let bytes = SerializeFactory::new("application/msgpack")
            .to_vec(&payload)
            .unwrap();
        let decoded: Payload = DeserializeFactory::new("application/msgpack")
            .from_slice(&bytes)
            .unwrap();
        assert_eq!(decoded, payload);
    }
}
