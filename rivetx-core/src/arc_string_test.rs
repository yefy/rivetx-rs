use crate::arc_string::ArcString;
use mysql_common::value::Value;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct TestStruct {
    #[serde(rename = "custom_string")]
    custom_string: ArcString,
}

/// Helper: builds an ArcString from a &'static str
fn s(s: &'static str) -> ArcString {
    ArcString::from(s)
}

// ────────── Construction / Default / Basic Access ──────────

#[test]
fn test_arc_string_new_default_and_basic_access() {
    let arc_string = ArcString::new("hello".to_string());
    assert_eq!(arc_string.as_str(), "hello");
    assert_eq!(arc_string.len(), 5);
    assert!(!arc_string.is_empty());

    let default = ArcString::default();
    assert_eq!(default.as_str(), "");
    assert_eq!(default.len(), 0);
    assert!(default.is_empty());
}

// ────────── From Various Sources ──────────

#[test]
fn test_arc_string_from_string() {
    let a = ArcString::from("hello".to_string());
    assert_eq!(a.as_str(), "hello");
}

#[test]
fn test_arc_string_from_static_str() {
    let a = ArcString::from("hello");
    assert_eq!(a.as_str(), "hello");
}

#[test]
fn test_arc_string_from_arc_str() {
    let shared: Arc<str> = Arc::from("hello");
    let a = ArcString::from(shared);
    assert_eq!(a.as_str(), "hello");
}

#[test]
fn test_arc_string_from_arc_string() {
    let shared: Arc<String> = Arc::new("hello".to_string());
    let a = ArcString::from(shared);
    assert_eq!(a.as_str(), "hello");
}

#[test]
fn test_arc_string_from_str() {
    let a = ArcString::from_str("hello");
    assert_eq!(a.as_str(), "hello");
}

#[test]
fn test_arc_string_from_cow_borrowed() {
    let cow: std::borrow::Cow<'_, str> = std::borrow::Cow::Borrowed("hello");
    let a = ArcString::from(cow);
    assert_eq!(a.as_str(), "hello");
}

#[test]
fn test_arc_string_from_cow_owned() {
    let cow: std::borrow::Cow<'_, str> = std::borrow::Cow::Owned("hello".to_string());
    let a = ArcString::from(cow);
    assert_eq!(a.as_str(), "hello");
}

#[test]
fn test_arc_string_empty_from_various_sources() {
    // Empty string should always return the same EMPTY_ARC_STR
    let a = ArcString::from("");
    assert_eq!(a.as_str(), "");
    assert_eq!(a.len(), 0);

    let b = ArcString::from("".to_string());
    assert_eq!(b.as_str(), "");
    assert_eq!(b.len(), 0);

    let c = ArcString::from_str("");
    assert_eq!(c.as_str(), "");
    assert_eq!(c.len(), 0);
}

// ────────── Display / Debug ──────────

#[test]
fn test_arc_string_display() {
    assert_eq!(format!("{}", s("hello")), "hello");
}

#[test]
fn test_arc_string_debug() {
    assert_eq!(format!("{:?}", s("hello")), "ArcString(\"hello\")");
}

// ────────── Deref / Indexing ──────────

#[test]
fn test_arc_string_deref_as_str() {
    let a = s("abcdefgh");
    assert_eq!(&a[..], "abcdefgh");
    assert_eq!(a.as_str(), "abcdefgh");
    assert_eq!(a.to_string(), "abcdefgh".to_string());
}

#[test]
fn test_arc_string_as_ref() {
    let a = s("hello");
    let _: &[u8] = a.as_ref();
    let _: &str = a.as_ref();
    assert_eq!(a.as_ref() as &str, "hello");
}

#[test]
fn test_arc_string_borrow() {
    use std::borrow::Borrow;
    let a = s("hello");
    let borrowed: &str = a.borrow();
    assert_eq!(borrowed, "hello");
}

// ────────── Clone ──────────

#[test]
fn test_arc_string_clone() {
    let a = s("hello world");
    let b = a.clone();
    assert_eq!(a, b);
    assert_eq!(a.as_str(), b.as_str());
    assert_eq!(a.len(), b.len());
}

// ────────── Equality / Ordering / Hash ──────────

#[test]
fn test_arc_string_eq() {
    let a = s("hello");
    let b = s("hello");
    let c = s("world");
    assert_eq!(a, b);
    assert_ne!(a, c);
    assert_eq!(a, "hello");
}

#[test]
fn test_arc_string_ord() {
    assert!(s("apple") < s("banana"));
    assert!(s("banana") > s("apple"));
    assert!(s("same") == s("same"));
}

#[test]
fn test_arc_string_hash_map() {
    let mut map: HashMap<ArcString, u32> = HashMap::new();

    map.insert(s("key1"), 10);
    map.insert(s("key2"), 20);

    assert_eq!(*map.get("key1").unwrap(), 10);
    assert_eq!(*map.get(&s("key1")).unwrap(), 10);
    assert_eq!(*map.get(&s("key2")).unwrap(), 20);

    // Overwrite
    map.insert(s("key1"), 30);
    assert_eq!(*map.get(&s("key1")).unwrap(), 30);
}

// ────────── Slice ──────────

#[test]
fn test_arc_string_slice_full() {
    let a = s("hello world");
    let slice = a.slice(..);
    assert_eq!(slice.as_str(), "hello world");
    assert_eq!(slice.len(), 11);
}

#[test]
fn test_arc_string_slice_partial() {
    let a = s("hello world");
    let slice = a.slice(6..11);
    assert_eq!(slice.as_str(), "world");
    assert_eq!(slice.len(), 5);
}

#[test]
fn test_arc_string_slice_middle() {
    let a = s("abcdefgh");
    let slice = a.slice(3..7);
    assert_eq!(slice.as_str(), "defg");
    assert_eq!(slice.len(), 4);
}

#[test]
fn test_arc_string_slice_nested() {
    let a = s("abcdefgh");
    let slice1 = a.slice(3..7); // "defg"
    let slice2 = slice1.slice(1..3); // "ef"
    assert_eq!(slice2.as_str(), "ef");
    assert_eq!(slice2.len(), 2);
}

#[test]
fn test_arc_string_slice_boundary_start() {
    let a = s("hello world");
    let slice = a.slice(0..1);
    assert_eq!(slice.as_str(), "h");
    assert_eq!(slice.len(), 1);
}

#[test]
fn test_arc_string_slice_boundary_end() {
    let a = s("hello world");
    let slice = a.slice(10..11);
    assert_eq!(slice.as_str(), "d");
    assert_eq!(slice.len(), 1);
}

#[test]
fn test_arc_string_slice_empty() {
    let a = s("hello world");
    let slice = a.slice(5..5);
    assert_eq!(slice.as_str(), "");
    assert_eq!(slice.len(), 0);
    assert!(slice.is_empty());
}

#[test]
fn test_arc_string_slice_range_inclusive() {
    let a = s("hello world");
    let slice = a.slice(0..=4);
    assert_eq!(slice.as_str(), "hello");
    assert_eq!(slice.len(), 5);
}

#[test]
fn test_arc_string_slice_from_start() {
    let a = s("hello world");
    let slice = a.slice(..5);
    assert_eq!(slice.as_str(), "hello");
}

#[test]
fn test_arc_string_slice_to_end() {
    let a = s("hello world");
    let slice = a.slice(6..);
    assert_eq!(slice.as_str(), "world");
}

// ────────── Trim ──────────

#[test]
fn test_arc_string_trim_no_change() {
    let a = s("hello");
    let trimmed = a.trim();
    assert_eq!(trimmed.as_str(), "hello");
}

#[test]
fn test_arc_string_trim_whitespace() {
    let a = s("  hello world  ");
    let trimmed = a.trim();
    assert_eq!(trimmed.as_str(), "hello world");
    assert_eq!(trimmed.len(), 11);
}

#[test]
fn test_arc_string_trim_all_whitespace() {
    let a = s("   ");
    let trimmed = a.trim();
    assert_eq!(trimmed.as_str(), "");
    assert!(trimmed.is_empty());
}

// ────────── Split ──────────

#[test]
fn test_arc_string_split() {
    let a = s("aaa aaa");
    let parts: Vec<&str> = a.as_str().split("aaa").collect();
    let arc_parts = a.split("aaa");
    for (i, part) in arc_parts.iter().enumerate() {
        assert_eq!(part.as_str(), parts[i]);
    }
}

#[test]
fn test_arc_string_split_complex() {
    let a = s(" aaa1 aaa313 aaarwr aaahth4342");
    let parts: Vec<&str> = a.as_str().split("aaa").collect();
    let arc_parts = a.split("aaa");
    for (i, part) in arc_parts.iter().enumerate() {
        assert_eq!(part.as_str(), parts[i]);
    }
}

#[test]
fn test_arc_string_split_no_delimiter() {
    let a = s("hello");
    let parts = a.split(",");
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].as_str(), "hello");
}

#[test]
fn test_arc_string_split_empty() {
    let a = s("");
    let parts = a.split(",");
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].as_str(), "");
}

// ────────── Push ──────────

#[test]
fn test_arc_string_push_str() {
    let mut a = s("hello");
    a.push_str(" world");
    assert_eq!(a.as_str(), "hello world");
}

#[test]
fn test_arc_string_push_str_empty() {
    let mut a = s("");
    a.push_str("hello");
    assert_eq!(a.as_str(), "hello");
}

// ────────── Large String / Large Slice ──────────

#[test]
fn test_arc_string_large_string() {
    let large = "a".repeat(1_000_000);
    let a = ArcString::from(large.clone());
    assert_eq!(a.len(), large.len());
    assert_eq!(a.as_str(), large);
    assert_eq!(a.to_string(), large);
}

#[test]
fn test_arc_string_large_slice() {
    let large = "a".repeat(1_000_000);
    let a = ArcString::from(large);
    let slice = a.slice(100_000..200_000);
    assert_eq!(slice.len(), 100_000);
    assert_eq!(slice.as_str(), "a".repeat(100_000));
}

// ────────── Serde ──────────

#[test]
fn test_arc_string_serde_roundtrip() {
    let arc_str = s("hello world");
    let data = TestStruct {
        custom_string: arc_str.clone(),
    };

    let json = serde_json::to_string(&data).unwrap();
    let parsed: TestStruct = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.custom_string, arc_str);
}

#[test]
fn test_arc_string_serde_empty() {
    let data = TestStruct {
        custom_string: s(""),
    };

    let json = serde_json::to_string(&data).unwrap();
    let parsed: TestStruct = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.custom_string.as_str(), "");
}

// ────────── Value Conversion ──────────

#[test]
fn test_arc_string_into_value() {
    let a = s("hello");
    let value: Value = a.into();
    assert_eq!(value, Value::Bytes(b"hello".to_vec()));
}

// ────────── RivetxString -> ArcString ──────────

#[test]
fn test_arc_string_from_rivetx_string() {
    use crate::rivetx_string::RivetxString;
    let rs = RivetxString::from("hello");
    let a = ArcString::from(rs);
    assert_eq!(a.as_str(), "hello");
}

// ────────── Clone Shares Underlying Data ──────────

#[test]
fn test_arc_string_clone_shares_data() {
    let a = s("hello world");
    let b = a.clone();
    assert_eq!(a.as_str(), b.as_str());
    // modifying b should not affect a (immutable, ensure logical independence)
    let mut b_mut = b;
    b_mut.push_str("!!");
    assert_eq!(a.as_str(), "hello world");
    assert_eq!(b_mut.as_str(), "hello world!!");
}

// ────────── Empty EMPTY_ARC_STR ──────────

#[test]
fn test_arc_string_empty_static_default() {
    let a = ArcString::default();
    let b = ArcString::from("");
    assert_eq!(a.as_str(), "");
    assert_eq!(b.as_str(), "");
    assert_eq!(a.len(), 0);
    assert_eq!(b.len(), 0);
}
