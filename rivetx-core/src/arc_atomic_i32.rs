use mysql_common::value::convert::FromValue;
use mysql_common::value::convert::FromValueError;
use mysql_common::value::Value;
use serde::de::{Deserialize, Deserializer};
use serde::ser::Serializer;
use serde::Serialize;
use std::convert::TryFrom;
use std::fmt;
use std::ops::Deref;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;

#[derive(Clone)]
pub struct ArcAtomicI32(pub Arc<AtomicI32>);

impl ArcAtomicI32 {
    pub fn new(d: i32) -> Self {
        ArcAtomicI32(Arc::new(AtomicI32::new(d)))
    }
    pub fn as_i32(&self) -> i32 {
        self.0.load(Ordering::Relaxed)
    }
}

impl Default for ArcAtomicI32 {
    #[inline]
    fn default() -> Self {
        Self::new(0)
    }
}

impl From<i32> for ArcAtomicI32 {
    #[inline]
    fn from(d: i32) -> Self {
        ArcAtomicI32::new(d)
    }
}

impl Deref for ArcAtomicI32 {
    type Target = Arc<AtomicI32>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Debug for ArcAtomicI32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Drain").field(&self.as_i32()).finish()
    }
}

impl fmt::Display for ArcAtomicI32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.as_i32(), f)
    }
}

impl TryFrom<Value> for ArcAtomicI32 {
    type Error = FromValueError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Bytes(bytes) => match std::str::from_utf8(&bytes) {
                Ok(f) => match f.parse::<i32>() {
                    Ok(i) => Ok(ArcAtomicI32::new(i as i32)),
                    _ => Err(FromValueError(Value::Bytes(bytes))),
                },
                _ => Err(FromValueError(Value::Bytes(bytes))),
            },
            Value::Int(i) => Ok(ArcAtomicI32::new(i as i32)),
            Value::UInt(i) => Ok(ArcAtomicI32::new(i as i32)),
            Value::Float(i) => Ok(ArcAtomicI32::new(i as i32)),
            Value::Double(i) => Ok(ArcAtomicI32::new(i as i32)),
            v => Err(FromValueError(v)),
        }
    }
}

impl FromValue for ArcAtomicI32 {
    type Intermediate = ArcAtomicI32;
}

impl From<ArcAtomicI32> for Value {
    fn from(d: ArcAtomicI32) -> Value {
        Value::Int(d.as_i32() as i64)
    }
}

impl Serialize for ArcAtomicI32 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_i32(self.as_i32())
    }
}

impl<'de> Deserialize<'de> for ArcAtomicI32 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = i32::deserialize(deserializer)?;
        Ok(ArcAtomicI32::new(s))
    }
}

#[cfg(test)]
pub mod tests {
    use super::ArcAtomicI32;
    use serde::{Deserialize, Serialize};

    //cargo test test_arc_atomic_i32 -- --nocapture
    //cargo test test_arc_atomic_i32 --release --no-default-features --features "rubic_test"  -- --nocapture
    #[tokio::test]
    async fn test_arc_atomic_i32() {
        #[derive(Clone, Debug, Serialize, Deserialize)]
        pub struct DataJson {
            pub send_count: ArcAtomicI32,
            pub recv_count: ArcAtomicI32,
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
