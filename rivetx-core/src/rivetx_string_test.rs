use crate::arc_string::ArcString;
use crate::rivetx_string::RivetxString;
use mysql_common::value::Value;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

fn owned(s: &str) -> RivetxString {
    RivetxString::from(s.to_string())
}

fn shared_str(s: &'static str) -> RivetxString {
    RivetxString::from(Arc::<str>::from(s))
}

fn shared_string(s: &'static str) -> RivetxString {
    RivetxString::from(Arc::new(s.to_string()))
}

fn arc_string(s: &'static str) -> RivetxString {
    RivetxString::from(ArcString::from(s.to_string()))
}

fn static_str(s: &'static str) -> RivetxString {
    RivetxString::from(s)
}

#[test]
fn test_rivetx_string_new_default_and_basic_access() {
    let a = RivetxString::new("hello".to_string());
    assert_eq!(a.as_str(), "hello");
    assert_eq!(a.len(), 5);
    assert!(!a.is_empty());

    let default = RivetxString::default();
    assert_eq!(default.as_str(), "");
    assert_eq!(default.len(), 0);
    assert!(default.is_empty());
}

#[test]
fn test_rivetx_string_from_variants() {
    assert_eq!(owned("hello").as_str(), "hello");
    assert_eq!(static_str("hello").as_str(), "hello");
    assert_eq!(shared_str("hello").as_str(), "hello");
    assert_eq!(shared_string("hello").as_str(), "hello");
    assert_eq!(arc_string("hello").as_str(), "hello");

    let cow_borrowed: Cow<'_, str> = Cow::Borrowed("hello");
    assert_eq!(RivetxString::from(cow_borrowed).as_str(), "hello");

    let cow_owned: Cow<'_, str> = Cow::Owned("hello".to_string());
    assert_eq!(RivetxString::from(cow_owned).as_str(), "hello");
}

#[test]
fn test_rivetx_string_display_and_debug() {
    let owned = owned("hello");
    let static_value = static_str("hello");

    assert_eq!(format!("{}", owned), "hello");
    assert_eq!(format!("{}", static_value), "hello");
    assert_eq!(format!("{:?}", owned), "Owned(\"hello\")");
    assert_eq!(format!("{:?}", static_str("hello")), "Static(\"hello\")");
}

#[test]
fn test_rivetx_string_equality_and_ordering() {
    let a = owned("apple");
    let b = static_str("apple");
    let c = owned("banana");

    assert_eq!(a, b);
    assert!(a < c);
    assert!(c > b);
    assert_eq!(a, "apple");
    assert_ne!(a, "banana");
}

#[test]
fn test_rivetx_string_hash_and_hashmap_lookup() {
    let mut map: HashMap<RivetxString, u32> = HashMap::new();
    map.insert(owned("key1"), 10);
    map.insert(static_str("key2"), 20);

    assert_eq!(*map.get("key1").unwrap(), 10);
    assert_eq!(*map.get("key2").unwrap(), 20);
    assert_eq!(*map.get(&owned("key1")).unwrap(), 10);
}

#[test]
fn test_rivetx_string_to_and_into_string() {
    let owned_value = owned("hello");
    assert_eq!(owned_value.to_string(), "hello".to_string());
    assert_eq!(owned_value.into_string(), "hello".to_string());

    let shared_value = static_str("hello");
    assert_eq!(shared_value.to_string(), "hello".to_string());
}

#[test]
fn test_rivetx_string_push_str_owned_and_non_owned() {
    let mut owned_value = owned("hello");
    owned_value.push_str(" world");
    assert_eq!(owned_value.as_str(), "hello world");

    let mut static_value = static_str("hello");
    static_value.push_str(" world");
    assert_eq!(static_value.as_str(), "hello world");

    let mut shared_value = shared_string("hello");
    shared_value.push_str(" world");
    assert_eq!(shared_value.as_str(), "hello world");
}

#[test]
fn test_rivetx_string_trim() {
    let value = owned("  hello  ");
    let trimmed = value.trim();
    assert_eq!(trimmed.as_str(), "hello");

    let already_trimmed = owned("hello");
    let cloned = already_trimmed.trim();
    assert_eq!(cloned.as_str(), "hello");
}

#[test]
fn test_rivetx_string_into_arc_string_and_clone_to_arc_string() {
    let owned_value = owned("hello");
    let as_arc = owned_value.into_arc_string();
    assert_eq!(as_arc.as_str(), "hello");

    let shared_value = shared_string("hello");
    let cloned = shared_value.clone_to_arc_string();
    assert_eq!(cloned.as_str(), "hello");
}

#[test]
fn test_rivetx_string_as_ref_and_borrow() {
    let value = owned("hello");
    let bytes: &[u8] = value.as_ref();
    assert_eq!(bytes, b"hello");

    let str_ref: &str = value.as_ref();
    assert_eq!(str_ref, "hello");

    let borrowed: &str = std::borrow::Borrow::borrow(&value);
    assert_eq!(borrowed, "hello");
}

#[test]
fn test_rivetx_string_value_conversion() {
    let value: Value = owned("hello").into();
    assert_eq!(value, Value::Bytes(b"hello".to_vec()));
}
