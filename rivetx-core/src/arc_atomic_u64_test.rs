use crate::arc_atomic_u64::ArcAtomicU64;
use mysql_common::value::Value;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::sync::atomic::Ordering;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DataJson {
    pub send_count: ArcAtomicU64,
    pub recv_count: ArcAtomicU64,
}

#[test]
fn test_arc_atomic_u64_new_default_and_as_u64() {
    let value = ArcAtomicU64::new(42);
    assert_eq!(value.as_u64(), 42);
    assert_eq!(ArcAtomicU64::default().as_u64(), 0);
    let from_into: ArcAtomicU64 = 7.into();
    assert_eq!(from_into.as_u64(), 7);
}

#[test]
fn test_arc_atomic_u64_deref_and_atomic_ops() {
    let value = ArcAtomicU64::new(10);
    assert_eq!(value.fetch_add(5, Ordering::SeqCst), 10);
    assert_eq!(value.as_u64(), 15);
}

#[test]
fn test_arc_atomic_u64_display_debug() {
    let value = ArcAtomicU64::new(123);
    assert_eq!(format!("{}", value), "123");
    assert_eq!(format!("{:?}", value), "Drain(123)");
}

#[test]
fn test_arc_atomic_u64_try_from_value_ok() {
    assert_eq!(ArcAtomicU64::try_from(Value::Int(1)).unwrap().as_u64(), 1);
    assert_eq!(ArcAtomicU64::try_from(Value::UInt(2)).unwrap().as_u64(), 2);
    assert_eq!(
        ArcAtomicU64::try_from(Value::Float(3.0)).unwrap().as_u64(),
        3
    );
    assert_eq!(
        ArcAtomicU64::try_from(Value::Double(4.0)).unwrap().as_u64(),
        4
    );
    assert_eq!(
        ArcAtomicU64::try_from(Value::Bytes(b"5".to_vec()))
            .unwrap()
            .as_u64(),
        5
    );
}

#[test]
fn test_arc_atomic_u64_try_from_value_invalid_bytes() {
    assert!(ArcAtomicU64::try_from(Value::Bytes(b"abc".to_vec())).is_err());
}

#[test]
fn test_arc_atomic_u64_value_conversion() {
    let value: Value = ArcAtomicU64::new(8).into();
    assert_eq!(value, Value::UInt(8));
}

#[test]
fn test_arc_atomic_u64_serde_roundtrip() {
    let data = DataJson {
        send_count: 1.into(),
        recv_count: 12.into(),
    };

    let json = serde_json::to_string(&data).unwrap();
    let parsed: DataJson = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.send_count.as_u64(), data.send_count.as_u64());
    assert_eq!(parsed.recv_count.as_u64(), data.recv_count.as_u64());
}
