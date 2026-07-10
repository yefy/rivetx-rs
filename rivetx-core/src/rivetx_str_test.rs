use crate::arc_string::ArcString;
use crate::rivetx_str::RivetxStr;
use crate::rivetx_string::RivetxString;
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

fn arc(s: &'static str) -> ArcString {
    ArcString::from(s)
}

// ────────── Construction / Default / Basic Access ──────────

#[test]
fn test_rivetx_str_default_and_basic_access() {
    let value = RivetxStr::from("hello");
    assert_eq!(value.as_str(), "hello");
    assert_eq!(value.len(), 5);
    assert!(!value.is_empty());
    assert_eq!(&*value, "hello");

    let default = RivetxStr::default();
    assert_eq!(default.as_str(), "");
    assert_eq!(default.len(), 0);
    assert!(default.is_empty());
    assert!(matches!(default, RivetxStr::Static("")));
}

#[test]
fn test_rivetx_str_from_str_and_static() {
    let local = String::from("hello");
    let borrowed = RivetxStr::from(local.as_str());
    assert!(matches!(borrowed, RivetxStr::Ref(_)));
    assert_eq!(borrowed.as_str(), "hello");

    let empty = RivetxStr::from("");
    assert!(matches!(empty, RivetxStr::Static("")));

    let static_value = RivetxStr::from_static("hello");
    assert!(matches!(static_value, RivetxStr::Static("hello")));
    assert_eq!(static_value.as_str(), "hello");
}

#[test]
fn test_rivetx_str_from_rivetx_string_variants() {
    assert_eq!(RivetxStr::from(&owned("hello")).as_str(), "hello");
    assert_eq!(RivetxStr::from(&static_str("hello")).as_str(), "hello");
    assert_eq!(RivetxStr::from(&shared_str("hello")).as_str(), "hello");
    assert_eq!(RivetxStr::from(&shared_string("hello")).as_str(), "hello");
    assert_eq!(RivetxStr::from(&arc_string("hello")).as_str(), "hello");

    assert!(matches!(
        RivetxStr::from(&owned("hello")),
        RivetxStr::RivetxStringRef(_)
    ));
    assert!(matches!(
        RivetxStr::from(&static_str("hello")),
        RivetxStr::RivetxStringRef(_)
    ));

    let owned_value = owned("hello");
    assert!(matches!(
        RivetxStr::from(owned_value),
        RivetxStr::RivetxString(_)
    ));

    let empty_rs = RivetxString::default();
    let empty = RivetxStr::from(&empty_rs);
    assert!(matches!(empty, RivetxStr::Static("")));
}

#[test]
fn test_rivetx_str_from_arc_string_variants() {
    let value = arc("hello");
    assert_eq!(RivetxStr::from(&value).as_str(), "hello");
    assert!(matches!(
        RivetxStr::from(&value),
        RivetxStr::ArcStringRef(_)
    ));

    let owned = RivetxStr::from(value.clone());
    assert!(matches!(owned, RivetxStr::ArcString(_)));
    assert_eq!(owned.as_str(), "hello");

    let empty_arc = ArcString::default();
    let empty = RivetxStr::from(&empty_arc);
    assert!(matches!(empty, RivetxStr::Static("")));
}

#[test]
fn test_rivetx_str_from_string_and_arc() {
    assert_eq!(RivetxStr::from("hello".to_string()).as_str(), "hello");
    assert_eq!(
        RivetxStr::from(Arc::<str>::from("hello")).as_str(),
        "hello"
    );
    assert_eq!(
        RivetxStr::from(Arc::new("hello".to_string())).as_str(),
        "hello"
    );
    assert!(matches!(
        RivetxStr::from("hello".to_string()),
        RivetxStr::RivetxString(_)
    ));
}

#[test]
fn test_rivetx_str_from_cow() {
    let borrowed: Cow<'_, str> = Cow::Borrowed("hello");
    let view = RivetxStr::from(borrowed);
    assert!(matches!(view, RivetxStr::Ref(_)));
    assert_eq!(view.as_str(), "hello");

    let owned: Cow<'_, str> = Cow::Owned("hello".to_string());
    let view = RivetxStr::from(owned);
    assert!(matches!(view, RivetxStr::RivetxString(_)));
    assert_eq!(view.as_str(), "hello");
}

// ────────── Display / Debug / Traits ──────────

#[test]
fn test_rivetx_str_display_and_debug() {
    let value = RivetxStr::from("hello");
    assert_eq!(format!("{}", value), "hello");
    assert_eq!(format!("{:?}", value), "RivetxStr(\"hello\")");
}

#[test]
fn test_rivetx_str_equality_and_ordering() {
    let a = RivetxStr::from("apple");
    let apple = static_str("apple");
    let b = RivetxStr::from(&apple);
    let c = RivetxStr::from("banana");

    assert_eq!(a, b);
    assert!(a < c);
    assert!(c > b);
    assert_eq!(a, "apple");
    assert_ne!(a, "banana");
}

#[test]
fn test_rivetx_str_hash_and_hashmap_lookup() {
    let mut map: HashMap<RivetxStr<'static>, u32> = HashMap::new();
    map.insert(RivetxStr::from("key1"), 10);
    map.insert(RivetxStr::from(static_str("key2")), 20);

    assert_eq!(*map.get("key1").unwrap(), 10);
    assert_eq!(*map.get("key2").unwrap(), 20);
    assert_eq!(*map.get(&RivetxStr::from("key1")).unwrap(), 10);
}

#[test]
fn test_rivetx_str_as_ref_and_borrow() {
    let value = RivetxStr::from("hello");
    assert_eq!(AsRef::<str>::as_ref(&value), "hello");
    assert_eq!(AsRef::<[u8]>::as_ref(&value), b"hello");
    assert_eq!(std::borrow::Borrow::<str>::borrow(&value), "hello");
}

// ────────── Conversion to RivetxString / ArcString ──────────

#[test]
fn test_rivetx_str_to_rivetx_string() {
    assert_eq!(RivetxStr::from("hello").to_rivetx_string().as_str(), "hello");
    assert_eq!(
        RivetxStr::from(&owned("hello")).to_rivetx_string().as_str(),
        "hello"
    );
    assert_eq!(
        RivetxStr::from(&shared_str("hello"))
            .to_rivetx_string()
            .as_str(),
        "hello"
    );
    assert_eq!(
        RivetxStr::from(&arc("hello")).to_rivetx_string().as_str(),
        "hello"
    );
    assert_eq!(
        RivetxStr::from(owned("hello")).to_rivetx_string().as_str(),
        "hello"
    );
    assert_eq!(
        RivetxStr::from(arc("hello")).to_rivetx_string().as_str(),
        "hello"
    );
}

#[test]
fn test_rivetx_str_into_rivetx_string() {
    let moved = RivetxStr::from(owned("hello")).into_rivetx_string();
    assert_eq!(moved.as_str(), "hello");

    let converted: RivetxString = RivetxStr::from("hello").into();
    assert_eq!(converted.as_str(), "hello");
}

#[test]
fn test_rivetx_str_to_arc_string() {
    assert_eq!(RivetxStr::from("hello").to_arc_string().as_str(), "hello");
    assert_eq!(
        RivetxStr::from(&owned("hello")).to_arc_string().as_str(),
        "hello"
    );
    assert_eq!(
        RivetxStr::from(&shared_str("hello"))
            .to_arc_string()
            .as_str(),
        "hello"
    );
    assert_eq!(
        RivetxStr::from(&shared_string("hello"))
            .to_arc_string()
            .as_str(),
        "hello"
    );
    assert_eq!(
        RivetxStr::from(&arc_string("hello"))
            .to_arc_string()
            .as_str(),
        "hello"
    );
    assert_eq!(
        RivetxStr::from(&arc("hello")).to_arc_string().as_str(),
        "hello"
    );
    assert_eq!(
        RivetxStr::from(arc("hello")).to_arc_string().as_str(),
        "hello"
    );
}

#[test]
fn test_rivetx_str_into_arc_string() {
    let moved = RivetxStr::from(arc("hello")).into_arc_string();
    assert_eq!(moved.as_str(), "hello");

    let from_arc_string_variant = RivetxStr::from(arc_string("hello")).into_arc_string();
    assert_eq!(from_arc_string_variant.as_str(), "hello");

    let converted: ArcString = RivetxStr::from("hello").into();
    assert_eq!(converted.as_str(), "hello");
}

#[test]
fn test_rivetx_str_clone_preserves_content() {
    let hello = shared_str("hello");
    let original = RivetxStr::from(&hello);
    let cloned = original.clone();
    assert_eq!(original, cloned);
    assert_eq!(cloned.as_str(), "hello");
}

// ────────── Zero-copy behavior ──────────

#[test]
fn test_rivetx_str_borrowed_sources_do_not_allocate_on_view() {
    let local = String::from("hello");
    let rs = shared_str("hello");
    let arc_value = arc("hello");

    assert!(matches!(RivetxStr::from(local.as_str()), RivetxStr::Ref(_)));
    assert!(matches!(RivetxStr::from(&rs), RivetxStr::RivetxStringRef(_)));
    assert!(matches!(
        RivetxStr::from(&arc_value),
        RivetxStr::ArcStringRef(_)
    ));

    let cow: Cow<'_, str> = Cow::Borrowed(local.as_str());
    assert!(matches!(RivetxStr::from(cow), RivetxStr::Ref(_)));
}

#[test]
fn test_rivetx_str_to_arc_string_reuses_shared_storage() {
    let shared = Arc::<str>::from("hello");
    let rs = RivetxString::from(Arc::clone(&shared));
    let view = RivetxStr::from(&rs);

    let converted = view.to_arc_string();
    assert_eq!(converted.as_str(), "hello");

    let cloned_view = view.clone();
    assert_eq!(cloned_view.to_arc_string().as_str(), "hello");
}
