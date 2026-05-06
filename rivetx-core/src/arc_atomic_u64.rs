use mysql_common::value::convert::FromValue;
use mysql_common::value::convert::FromValueError;
use mysql_common::value::Value;
use serde::de::{Deserialize, Deserializer};
use serde::ser::Serializer;
use serde::Serialize;
use std::convert::TryFrom;
use std::fmt;
use std::ops::Deref;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Clone)]
pub struct ArcAtomicU64(pub Arc<AtomicU64>);

impl ArcAtomicU64 {
    pub fn new(d: u64) -> Self {
        ArcAtomicU64(Arc::new(AtomicU64::new(d)))
    }
    pub fn as_u64(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}

impl Default for ArcAtomicU64 {
    #[inline]
    fn default() -> Self {
        Self::new(0)
    }
}

impl From<u64> for ArcAtomicU64 {
    #[inline]
    fn from(d: u64) -> Self {
        ArcAtomicU64::new(d)
    }
}

impl Deref for ArcAtomicU64 {
    type Target = Arc<AtomicU64>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Debug for ArcAtomicU64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Drain").field(&self.as_u64()).finish()
    }
}

impl fmt::Display for ArcAtomicU64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.as_u64(), f)
    }
}

impl TryFrom<Value> for ArcAtomicU64 {
    type Error = FromValueError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Bytes(bytes) => match std::str::from_utf8(&bytes) {
                Ok(f) => match f.parse::<u64>() {
                    Ok(i) => Ok(ArcAtomicU64::new(i as u64)),
                    _ => Err(FromValueError(Value::Bytes(bytes))),
                },
                _ => Err(FromValueError(Value::Bytes(bytes))),
            },
            Value::Int(i) => Ok(ArcAtomicU64::new(i as u64)),
            Value::UInt(i) => Ok(ArcAtomicU64::new(i as u64)),
            Value::Float(i) => Ok(ArcAtomicU64::new(i as u64)),
            Value::Double(i) => Ok(ArcAtomicU64::new(i as u64)),
            v => Err(FromValueError(v)),
        }
    }
}

impl FromValue for ArcAtomicU64 {
    type Intermediate = ArcAtomicU64;
}

impl From<ArcAtomicU64> for Value {
    fn from(d: ArcAtomicU64) -> Value {
        Value::UInt(d.as_u64() as u64)
    }
}

impl Serialize for ArcAtomicU64 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(self.as_u64())
    }
}

impl<'de> Deserialize<'de> for ArcAtomicU64 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = u64::deserialize(deserializer)?;
        Ok(ArcAtomicU64::new(s))
    }
}

#[cfg(test)]
pub mod tests {
    use super::ArcAtomicU64;
    use serde::{Deserialize, Serialize};

    //cargo test test_arc_atomic_u64 -- --nocapture
    //cargo test test_arc_atomic_u64 --release --no-default-features --features "rubic_test"  -- --nocapture
    #[tokio::test]
    async fn test_arc_atomic_u64() {
        #[derive(Clone, Debug, Serialize, Deserialize)]
        pub struct DataJson {
            pub send_count: ArcAtomicU64,
            pub recv_count: ArcAtomicU64,
        }

        let data = DataJson {
            send_count: 1.into(),
            recv_count: 12.into(),
        };
        println!("data:{:?}", data);
        let str = serde_json::to_string(&data).unwrap();
        println!("str:{:?}", str);
        let data_result: DataJson = serde_json::from_str(&str).unwrap();
        println!("data_result:{:?}", data_result);
    }
}
