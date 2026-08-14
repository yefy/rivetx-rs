#[cfg(test)]
mod tests {
    use crate::memory_cache::MemoryCache;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_memory_cache_basic_string_operations() {
        let cache = MemoryCache::new(32);

        cache.set("name", "alice", Some(Duration::from_secs(5)));

        assert_eq!(cache.get("name"), Some("alice".to_string()));
        assert!(cache.exists("name"));
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.strlen("name"), 5);
        assert!(cache.ttl("name") > 0);

        assert!(cache.set_ttl("name", Some(Duration::from_secs(2))));
        assert!(cache.ttl("name") <= 2);

        assert_eq!(cache.del(["name", "missing"]), 1);
        assert_eq!(cache.get("name"), None);
        assert!(!cache.exists("name"));
    }

    #[test]
    fn test_memory_cache_setnx_getset_incr_and_ttl() {
        let cache = MemoryCache::new(32);

        assert!(cache.setnx("counter", "7"));
        assert!(!cache.setnx("counter", "9"));
        assert_eq!(cache.getset("counter", "10"), Some("7".to_string()));

        assert_eq!(cache.incrby("counter", 5).unwrap(), 15);
        assert_eq!(cache.get("counter"), Some("15".to_string()));

        cache.set("float_key", "1.5", Some(Duration::from_secs(3)));
        assert!((cache.incrbyfloat("float_key", 2.5).unwrap() - 4.0).abs() < f64::EPSILON);
        assert!(cache.ttl("float_key") > 0);
    }

    #[test]
    fn test_memory_cache_expiration() {
        let cache = MemoryCache::new(8);
        cache.set("exp", "value", Some(Duration::from_millis(50)));

        assert_eq!(cache.get("exp"), Some("value".to_string()));
        thread::sleep(Duration::from_millis(120));

        assert_eq!(cache.get("exp"), None);
        assert_eq!(cache.ttl("exp"), -2);
    }

    #[test]
    fn test_memory_cache_hash_table_basic_operations() {
        let cache = MemoryCache::new(32);

        assert!(cache.hset("users", "alice", "admin"));
        assert!(!cache.hsetnx("users", "alice", "root"));
        assert!(cache.hsetnx("users", "bob", "user"));

        assert_eq!(cache.hget("users", "alice"), Some("admin".to_string()));
        assert_eq!(cache.hlen("users"), 2);
        assert_eq!(cache.hstrlen("users", "alice"), 5);
        assert_eq!(cache.hmget("users", &["alice", "bob", "ghost"]), vec![Some("admin".to_string()), Some("user".to_string()), None]);

        let mut expected = std::collections::HashMap::new();
        expected.insert("alice".to_string(), "admin".to_string());
        expected.insert("bob".to_string(), "user".to_string());
        assert_eq!(cache.hgetall("users"), expected);

        let mut keys = cache.hkeys("users");
        keys.sort();
        assert_eq!(keys, vec!["alice".to_string(), "bob".to_string()]);

        let mut values = cache.hvals("users");
        values.sort();
        assert_eq!(values, vec!["admin".to_string(), "user".to_string()]);

        assert_eq!(cache.hdel("users", &["alice", "ghost"]), 1);
        assert!(!cache.hexists("users", "alice"));
        assert!(cache.hexists("users", "bob"));

        assert_eq!(cache.hincrby("users", "score", 5).unwrap(), 5);
        assert_eq!(cache.hincrbyfloat("users", "ratio", 1.5).unwrap(), 1.5);
        assert!(cache.hexists_table("users"));
        assert_eq!(cache.table_count(), 1);

        assert!(cache.hset_ttl("users", Some(Duration::from_secs(3))));
        assert!(cache.httl("users") > 0);

        assert!(cache.hdel_table("users"));
        assert!(!cache.hexists_table("users"));
        assert!(cache.hgetall("users").is_empty());
        assert_eq!(cache.table_count(), 0);
    }
}
