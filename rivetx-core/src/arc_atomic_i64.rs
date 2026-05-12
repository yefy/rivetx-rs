use mysql_common::value::convert::FromValue;
use mysql_common::value::convert::FromValueError;
use mysql_common::value::Value;
use serde::de::{Deserialize, Deserializer};
use serde::ser::Serializer;
use serde::Serialize;
use std::convert::TryFrom;
use std::fmt;
use std::ops::Deref;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

#[derive(Clone)]
pub struct ArcAtomicI64(pub Arc<AtomicI64>);

impl ArcAtomicI64 {
    pub fn new(d: i64) -> Self {
        ArcAtomicI64(Arc::new(AtomicI64::new(d)))
    }
    pub fn as_i64(&self) -> i64 {
        self.0.load(Ordering::Relaxed)
    }
}

impl Default for ArcAtomicI64 {
    #[inline]
    fn default() -> Self {
        Self::new(0)
    }
}

impl From<i64> for ArcAtomicI64 {
    #[inline]
    fn from(d: i64) -> Self {
        ArcAtomicI64::new(d)
    }
}

impl Deref for ArcAtomicI64 {
    type Target = Arc<AtomicI64>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Debug for ArcAtomicI64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Drain").field(&self.as_i64()).finish()
    }
}

impl fmt::Display for ArcAtomicI64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.as_i64(), f)
    }
}

impl TryFrom<Value> for ArcAtomicI64 {
    type Error = FromValueError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Bytes(bytes) => match std::str::from_utf8(&bytes) {
                Ok(f) => match f.parse::<i64>() {
                    Ok(i) => Ok(ArcAtomicI64::new(i as i64)),
                    _ => Err(FromValueError(Value::Bytes(bytes))),
                },
                _ => Err(FromValueError(Value::Bytes(bytes))),
            },
            Value::Int(i) => Ok(ArcAtomicI64::new(i as i64)),
            Value::UInt(i) => Ok(ArcAtomicI64::new(i as i64)),
            Value::Float(i) => Ok(ArcAtomicI64::new(i as i64)),
            Value::Double(i) => Ok(ArcAtomicI64::new(i as i64)),
            v => Err(FromValueError(v)),
        }
    }
}

impl FromValue for ArcAtomicI64 {
    type Intermediate = ArcAtomicI64;
}

impl From<ArcAtomicI64> for Value {
    fn from(d: ArcAtomicI64) -> Value {
        Value::Int(d.as_i64() as i64)
    }
}

impl Serialize for ArcAtomicI64 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_i64(self.as_i64())
    }
}

impl<'de> Deserialize<'de> for ArcAtomicI64 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = i64::deserialize(deserializer)?;
        Ok(ArcAtomicI64::new(s))
    }
}
