use crate::arc_atomic_u32::ArcAtomicU32;
use mysql_common::value::Value;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::sync::atomic::Ordering;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DataJson {
    pub send_count: ArcAtomicU32,
    pub recv_count: ArcAtomicU32,
}

#[test]
fn test_arc_atomic_u32_new_default_and_as_u32() {
    let value = ArcAtomicU32::new(42);
    assert_eq!(value.as_u32(), 42);
    assert_eq!(ArcAtomicU32::default().as_u32(), 0);
    let from_into: ArcAtomicU32 = 7.into();
    assert_eq!(from_into.as_u32(), 7);
}

#[test]
fn test_arc_atomic_u32_deref_and_atomic_ops() {
    let value = ArcAtomicU32::new(10);
    assert_eq!(value.fetch_add(5, Ordering::SeqCst), 10);
    assert_eq!(value.as_u32(), 15);
}

#[test]
fn test_arc_atomic_u32_display_debug() {
    let value = ArcAtomicU32::new(123);
    assert_eq!(format!("{}", value), "123");
    assert_eq!(format!("{:?}", value), "Drain(123)");
}

#[test]
fn test_arc_atomic_u32_try_from_value_ok() {
    assert_eq!(ArcAtomicU32::try_from(Value::Int(1)).unwrap().as_u32(), 1);
    assert_eq!(ArcAtomicU32::try_from(Value::UInt(2)).unwrap().as_u32(), 2);
    assert_eq!(
        ArcAtomicU32::try_from(Value::Float(3.0)).unwrap().as_u32(),
        3
    );
    assert_eq!(
        ArcAtomicU32::try_from(Value::Double(4.0)).unwrap().as_u32(),
        4
    );
    assert_eq!(
        ArcAtomicU32::try_from(Value::Bytes(b"5".to_vec()))
            .unwrap()
            .as_u32(),
        5
    );
}

#[test]
fn test_arc_atomic_u32_try_from_value_invalid_bytes() {
    assert!(ArcAtomicU32::try_from(Value::Bytes(b"abc".to_vec())).is_err());
}

#[test]
fn test_arc_atomic_u32_value_conversion() {
    let value: Value = ArcAtomicU32::new(8).into();
    assert_eq!(value, Value::Int(8));
}

#[test]
fn test_arc_atomic_u32_serde_roundtrip() {
    let data = DataJson {
        send_count: 1.into(),
        recv_count: 12.into(),
    };

    let json = serde_json::to_string(&data).unwrap();
    let parsed: DataJson = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.send_count.as_u32(), data.send_count.as_u32());
    assert_eq!(parsed.recv_count.as_u32(), data.recv_count.as_u32());
}
