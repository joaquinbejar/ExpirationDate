//! Hand-written serde impls for [`ExpirationDate`].
//!
//! The wire shape is a single-entry map: `{"days": <f64>}` or
//! `{"datetime": "<RFC3339>"}`. That shape is a semver contract — changing
//! the tag or payload layout is a breaking change to every persisted value.

use crate::ExpirationDate;
use chrono::{DateTime, Utc};
use positive::Positive;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

impl Serialize for ExpirationDate {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeMap;
        let mut state = serializer.serialize_map(Some(1))?;
        match self {
            Self::Days(days) => state.serialize_entry("days", &days.to_f64())?,
            Self::DateTime(dt) => {
                state.serialize_entry("datetime", &dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())?
            }
        }
        state.end()
    }
}

impl<'de> Deserialize<'de> for ExpirationDate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{MapAccess, Visitor};
        struct ExVisitor;
        impl<'de> Visitor<'de> for ExVisitor {
            type Value = ExpirationDate;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("struct ExpirationDate")
            }
            fn visit_map<V>(self, mut map: V) -> Result<ExpirationDate, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut d = None;
                let mut t = None;
                while let Some(k) = map.next_key::<String>()? {
                    match k.as_str() {
                        "days" => {
                            if d.is_some() {
                                return Err(serde::de::Error::duplicate_field("days"));
                            }
                            d = Some(map.next_value::<f64>()?);
                        }
                        "datetime" => {
                            if t.is_some() {
                                return Err(serde::de::Error::duplicate_field("datetime"));
                            }
                            t = Some(map.next_value::<String>()?);
                        }
                        _ => {
                            return Err(serde::de::Error::unknown_field(&k, &["days", "datetime"]));
                        }
                    }
                }
                match (d, t) {
                    (Some(v), _) => Ok(ExpirationDate::Days(
                        Positive::new(v).map_err(serde::de::Error::custom)?,
                    )),
                    (_, Some(v)) => Ok(ExpirationDate::DateTime(
                        DateTime::parse_from_rfc3339(&v)
                            .map_err(serde::de::Error::custom)?
                            .with_timezone(&Utc),
                    )),
                    _ => Err(serde::de::Error::missing_field("days or datetime")),
                }
            }
        }
        deserializer.deserialize_struct("ExpirationDate", &["days", "datetime"], ExVisitor)
    }
}
