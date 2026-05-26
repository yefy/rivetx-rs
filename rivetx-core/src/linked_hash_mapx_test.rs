use crate::linked_hash_mapx::LinkedHashMapx;

// ────────── Construction ──────────

#[test]
fn test_linked_hash_mapx_new_empty() {
    let map = LinkedHashMapx::<i32, i32>::new(3);
    assert_eq!(map.max_size, 3);
    assert_eq!(map.hash_map.len(), 0);
}

#[test]
fn test_linked_hash_mapx_with_capacity() {
    let map = LinkedHashMapx::<&str, i32>::with_capacity(5, 10);
    assert_eq!(map.max_size, 5);
    assert_eq!(map.hash_map.len(), 0);
}

// ────────── Insert (basic) ──────────

#[test]
fn test_linked_hash_mapx_insert_new_key_returns_none() {
    let mut map = LinkedHashMapx::new(10);
    assert_eq!(map.insert("a", 1), None);
    assert_eq!(map.hash_map.get("a"), Some(&1));
    assert_eq!(map.hash_map.len(), 1);
}

#[test]
fn test_linked_hash_mapx_insert_update_returns_old_value() {
    let mut map = LinkedHashMapx::new(10);
    assert_eq!(map.insert("a", 1), None);
    assert_eq!(map.insert("a", 2), Some(1));
    assert_eq!(map.hash_map.get("a"), Some(&2));
    assert_eq!(map.hash_map.len(), 1);
}

// ────────── max_size eviction ──────────

#[test]
fn test_linked_hash_mapx_evicts_oldest_when_exceeds_max_size() {
    let mut map = LinkedHashMapx::new(2);
    map.insert(1, "one");
    map.insert(2, "two");
    map.insert(3, "three");

    assert_eq!(map.hash_map.len(), 2);
    assert!(!map.hash_map.contains_key(&1));
    assert!(map.hash_map.contains_key(&2));
    assert!(map.hash_map.contains_key(&3));
    assert_eq!(map.hash_map.get(&2), Some(&"two"));
    assert_eq!(map.hash_map.get(&3), Some(&"three"));
}

#[test]
fn test_linked_hash_mapx_eviction_preserves_insertion_order() {
    let mut map = LinkedHashMapx::new(3);
    for i in 1..=5 {
        map.insert(i, i * 10);
    }

    let keys: Vec<i32> = map.hash_map.keys().copied().collect();
    assert_eq!(keys, vec![3, 4, 5]);
}

#[test]
fn test_linked_hash_mapx_max_size_one() {
    let mut map = LinkedHashMapx::new(1);
    map.insert("a", 1);
    assert_eq!(map.hash_map.len(), 1);
    assert_eq!(map.hash_map.get("a"), Some(&1));

    map.insert("b", 2);
    assert_eq!(map.hash_map.len(), 1);
    assert!(!map.hash_map.contains_key("a"));
    assert_eq!(map.hash_map.get("b"), Some(&2));
}

#[test]
fn test_linked_hash_mapx_under_max_size_no_eviction() {
    let mut map = LinkedHashMapx::new(5);
    for i in 0..4 {
        map.insert(i, i);
    }
    assert_eq!(map.hash_map.len(), 4);
    for i in 0..4 {
        assert_eq!(map.hash_map.get(&i), Some(&i));
    }
}

#[test]
fn test_linked_hash_mapx_update_existing_does_not_evict() {
    let mut map = LinkedHashMapx::new(2);
    map.insert("a", 1);
    map.insert("b", 2);
    assert_eq!(map.insert("a", 10), Some(1));

    assert_eq!(map.hash_map.len(), 2);
    assert_eq!(map.hash_map.get("a"), Some(&10));
    assert_eq!(map.hash_map.get("b"), Some(&2));
}

// ────────── max_size = 0 (no eviction) ──────────

#[test]
fn test_linked_hash_mapx_max_size_zero_no_eviction() {
    let mut map = LinkedHashMapx::new(0);
    for i in 0..10 {
        map.insert(i, i);
    }
    assert_eq!(map.hash_map.len(), 10);
    for i in 0..10 {
        assert_eq!(map.hash_map.get(&i), Some(&i));
    }
}

// ────────── Repeated evictions ──────────

#[test]
fn test_linked_hash_mapx_repeated_evictions() {
    let mut map = LinkedHashMapx::new(2);
    map.insert("a", 1);
    map.insert("b", 2);
    map.insert("c", 3);
    map.insert("d", 4);

    assert_eq!(map.hash_map.len(), 2);
    assert!(!map.hash_map.contains_key("a"));
    assert!(!map.hash_map.contains_key("b"));
    assert_eq!(map.hash_map.get("c"), Some(&3));
    assert_eq!(map.hash_map.get("d"), Some(&4));

    let keys: Vec<&str> = map.hash_map.keys().copied().collect();
    assert_eq!(keys, vec!["c", "d"]);
}

// ────────── try_insert ──────────

#[test]
fn test_linked_hash_mapx_try_insert_new_key_returns_true() {
    let mut map = LinkedHashMapx::new(10);
    assert!(map.try_insert("a", 1));
    assert_eq!(map.hash_map.get("a"), Some(&1));
    assert_eq!(map.hash_map.len(), 1);
}

#[test]
fn test_linked_hash_mapx_try_insert_existing_key_returns_false() {
    let mut map = LinkedHashMapx::new(10);
    assert!(map.try_insert("a", 1));
    assert!(!map.try_insert("a", 2));
    assert_eq!(map.hash_map.get("a"), Some(&1));
    assert_eq!(map.hash_map.len(), 1);
}

#[test]
fn test_linked_hash_mapx_try_insert_evicts_oldest_when_exceeds_max_size() {
    let mut map = LinkedHashMapx::new(2);
    assert!(map.try_insert(1, "one"));
    assert!(map.try_insert(2, "two"));
    assert!(map.try_insert(3, "three"));

    assert_eq!(map.hash_map.len(), 2);
    assert!(!map.hash_map.contains_key(&1));
    assert!(map.hash_map.contains_key(&2));
    assert!(map.hash_map.contains_key(&3));
}

#[test]
fn test_linked_hash_mapx_try_insert_existing_key_does_not_evict() {
    let mut map = LinkedHashMapx::new(2);
    assert!(map.try_insert("a", 1));
    assert!(map.try_insert("b", 2));
    assert!(!map.try_insert("a", 10));

    assert_eq!(map.hash_map.len(), 2);
    assert_eq!(map.hash_map.get("a"), Some(&1));
    assert_eq!(map.hash_map.get("b"), Some(&2));
}

#[test]
fn test_linked_hash_mapx_try_insert_max_size_zero_no_eviction() {
    let mut map = LinkedHashMapx::new(0);
    for i in 0..10 {
        assert!(map.try_insert(i, i));
    }
    assert_eq!(map.hash_map.len(), 10);
    for i in 0..10 {
        assert_eq!(map.hash_map.get(&i), Some(&i));
    }
}
