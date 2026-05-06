use mysql_common::value::convert::FromValue;
use mysql_common::value::convert::FromValueError;
use mysql_common::value::Value;
use serde::de::{Deserialize, Deserializer};
use serde::ser::Serializer;
use serde::Serialize;
use std::convert::TryFrom;
use std::fmt;
use std::ops::Deref;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

#[derive(Clone)]
pub struct ArcAtomicU32(pub Arc<AtomicU32>);

impl ArcAtomicU32 {
    pub fn new(d: u32) -> Self {
        ArcAtomicU32(Arc::new(AtomicU32::new(d)))
    }
    pub fn as_u32(&self) -> u32 {
        self.0.load(Ordering::Relaxed)
    }
}

impl Default for ArcAtomicU32 {
    #[inline]
    fn default() -> Self {
        Self::new(0)
    }
}

impl From<u32> for ArcAtomicU32 {
    #[inline]
    fn from(d: u32) -> Self {
        ArcAtomicU32::new(d)
    }
}

impl Deref for ArcAtomicU32 {
    type Target = Arc<AtomicU32>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Debug for ArcAtomicU32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Drain").field(&self.as_u32()).finish()
    }
}

impl fmt::Display for ArcAtomicU32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.as_u32(), f)
    }
}

impl TryFrom<Value> for ArcAtomicU32 {
    type Error = FromValueError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Bytes(bytes) => match std::str::from_utf8(&bytes) {
                Ok(f) => match f.parse::<u32>() {
                    Ok(i) => Ok(ArcAtomicU32::new(i as u32)),
                    _ => Err(FromValueError(Value::Bytes(bytes))),
                },
                _ => Err(FromValueError(Value::Bytes(bytes))),
            },
            Value::Int(i) => Ok(ArcAtomicU32::new(i as u32)),
            Value::UInt(i) => Ok(ArcAtomicU32::new(i as u32)),
            Value::Float(i) => Ok(ArcAtomicU32::new(i as u32)),
            Value::Double(i) => Ok(ArcAtomicU32::new(i as u32)),
            v => Err(FromValueError(v)),
        }
    }
}

impl FromValue for ArcAtomicU32 {
    type Intermediate = ArcAtomicU32;
}

impl From<ArcAtomicU32> for Value {
    fn from(d: ArcAtomicU32) -> Value {
        Value::Int(d.as_u32() as i64)
    }
}

impl Serialize for ArcAtomicU32 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u32(self.as_u32())
    }
}

impl<'de> Deserialize<'de> for ArcAtomicU32 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = u32::deserialize(deserializer)?;
        Ok(ArcAtomicU32::new(s))
    }
}

#[cfg(test)]
pub mod tests {
    use super::ArcAtomicU32;
    use serde::{Deserialize, Serialize};

    //cargo test test_arc_atomic_u32 -- --nocapture
    //cargo test test_arc_atomic_u32 --release --no-default-features --features "rubic_test"  -- --nocapture
    #[tokio::test]
    async fn test_arc_atomic_u32() {
        #[derive(Clone, Debug, Serialize, Deserialize)]
        pub struct DataJson {
            pub send_count: ArcAtomicU32,
            pub recv_count: ArcAtomicU32,
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
