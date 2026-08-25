use moka::sync::Cache;
use moka::Expiry;

use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
    sync::{Arc, Mutex, RwLock},
    time::{Duration, Instant},
};

//
// ============================================================
// Shard 数量
// ============================================================
//
// u8::MAX == 255
//
// shard index:
//
// hash(key) % 255
//
// 得到：
// 0 .. 254
//
// 如果你真正想要 256 个 shard，应该使用 256。
// 这里按照你的要求使用 u8::MAX。
//
const COMMAND_LOCK_COUNT: usize = u8::MAX as usize;

//
// ============================================================
// ValueWithTtl
// ============================================================
//

#[derive(Debug, Clone)]
struct ValueWithTtl<V> {
    value: V,

    //
    // 绝对过期时间
    //
    // None = 永不过期
    //
    expires_at: Option<Instant>,
}

impl<V> ValueWithTtl<V> {
    fn new(value: V, ttl: Option<Duration>) -> Self {
        let expires_at = ttl.map(|ttl| Instant::now() + ttl);

        Self { value, expires_at }
    }

    fn persistent(value: V) -> Self {
        Self {
            value,
            expires_at: None,
        }
    }
    #[allow(dead_code)]
    fn remaining_ttl(&self) -> Option<Duration> {
        self.expires_at
            .map(|expires_at| expires_at.saturating_duration_since(Instant::now()))
    }
}

//
// ============================================================
// Dynamic Expiry
// ============================================================
//

struct DynamicExpiry;

impl<K, V> Expiry<K, ValueWithTtl<V>> for DynamicExpiry {
    //
    // 第一次创建
    //
    fn expire_after_create(
        &self,
        _key: &K,
        value: &ValueWithTtl<V>,
        created_at: Instant,
    ) -> Option<Duration> {
        value
            .expires_at
            .map(|expires_at| expires_at.saturating_duration_since(created_at))
    }

    //
    // 更新
    //
    fn expire_after_update(
        &self,
        _key: &K,
        value: &ValueWithTtl<V>,
        updated_at: Instant,
        _duration_until_expiry: Option<Duration>,
    ) -> Option<Duration> {
        value
            .expires_at
            .map(|expires_at| expires_at.saturating_duration_since(updated_at))
    }
}

//
// ============================================================
// Hash Table
// ============================================================
//

type HashTable = Arc<RwLock<HashMap<String, String>>>;

//
// ============================================================
// MemoryCache
// ============================================================
//

pub struct MemoryCache {
    //
    // 普通 KV
    //
    simple_cache: Cache<String, ValueWithTtl<String>>,

    //
    // Hash Table
    //
    // table -> Arc<RwLock<HashMap>>
    //
    table_cache: Cache<String, ValueWithTtl<HashTable>>,

    //
    // 255 个 command lock
    //
    // 不再是：
    //
    // Mutex<()>
    //
    // 而是：
    //
    // [Mutex<()>; 255]
    //
    command_locks: Box<[Mutex<()>; COMMAND_LOCK_COUNT]>,
}

//
// ============================================================
// MemoryCache
// ============================================================
//

impl MemoryCache {
    pub fn new(max_capacity: u64) -> Self {
        let simple_cache = Cache::builder()
            .max_capacity(max_capacity)
            .expire_after(DynamicExpiry)
            .build();

        let table_cache = Cache::builder()
            .max_capacity(max_capacity)
            .expire_after(DynamicExpiry)
            .build();

        //
        // 初始化 255 个 Mutex
        //
        let command_locks = Box::new(std::array::from_fn(|_| Mutex::new(())));

        Self {
            simple_cache,
            table_cache,
            command_locks,
        }
    }

    //
    // ========================================================
    // Hash
    // ========================================================
    //

    #[inline]
    fn hash_key(key: &str) -> usize {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);

        //
        // 255 个 shard
        //
        (hasher.finish() as usize) % COMMAND_LOCK_COUNT
    }

    //
    // ========================================================
    // 获取 command lock
    // ========================================================
    //

    #[inline]
    fn command_lock(&self, key: &str) -> &Mutex<()> {
        let index = Self::hash_key(key);

        &self.command_locks[index]
    }

    //
    // ========================================================
    // SET
    // ========================================================
    //
    // SET 本身是单次 Moka insert。
    //
    // 不需要 command lock。
    //
    // SET 会覆盖旧值，并清除旧 TTL。
    //
    // ttl = None：
    //
    // 永不过期。
    //
    pub fn set(&self, key: &str, value: &str, ttl: Option<Duration>) {
        self.simple_cache
            .insert(key.to_string(), ValueWithTtl::new(value.to_string(), ttl));
    }

    //
    // ========================================================
    // SETEX
    // ========================================================
    //

    pub fn setex(&self, key: &str, seconds: u64, value: &str) {
        self.set(key, value, Some(Duration::from_secs(seconds)));
    }

    //
    // ========================================================
    // SETNX
    // ========================================================
    //
    // 必须：
    //
    // 判断不存在
    // +
    // insert
    //
    // 两个操作必须在同一个 shard lock 中。
    //
    pub fn setnx(&self, key: &str, value: &str) -> bool {
        let lock = self.command_lock(key);
        let _guard = lock.lock().unwrap();

        if self.simple_cache.contains_key(key) {
            return false;
        }

        self.simple_cache
            .insert(key.to_string(), ValueWithTtl::persistent(value.to_string()));

        true
    }

    //
    // ========================================================
    // GET
    // ========================================================
    //
    // 不需要 command lock。
    //
    pub fn get(&self, key: &str) -> Option<String> {
        self.simple_cache.get(key).map(|v| v.value.clone())
    }

    //
    // ========================================================
    // GETSET
    // ========================================================
    //
    // get + set 必须原子。
    //
    pub fn getset(&self, key: &str, value: &str) -> Option<String> {
        let lock = self.command_lock(key);
        let _guard = lock.lock().unwrap();

        let old = self.simple_cache.get(key).map(|v| v.value.clone());

        //
        // Redis GETSET：
        //
        // 新 value 不带旧 TTL。
        //
        self.simple_cache
            .insert(key.to_string(), ValueWithTtl::persistent(value.to_string()));

        old
    }

    //
    // ========================================================
    // SET_TTL
    // ========================================================
    //
    // 修改 TTL，不修改 value。
    //
    pub fn set_ttl(&self, key: &str, ttl: Option<Duration>) -> bool {
        let lock = self.command_lock(key);
        let _guard = lock.lock().unwrap();

        let old = match self.simple_cache.get(key) {
            Some(v) => v,
            None => return false,
        };

        self.simple_cache
            .insert(key.to_string(), ValueWithTtl::new(old.value.clone(), ttl));

        true
    }

    //
    // ========================================================
    // TTL
    // ========================================================
    //
    // -2 = key 不存在
    // -1 = 永不过期
    // >=0 = 剩余秒数
    //
    pub fn ttl(&self, key: &str) -> i64 {
        match self.simple_cache.get(key) {
            None => -2,

            Some(value) => match value.expires_at {
                None => -1,

                Some(expires_at) => expires_at
                    .saturating_duration_since(Instant::now())
                    .as_secs() as i64,
            },
        }
    }

    //
    // ========================================================
    // DEL
    // ========================================================
    //
    // 多 key DEL。
    //
    // 为了保证：
    //
    // contains + invalidate
    //
    // 期间不会被对应的 SETNX / GETSET 等操作插入，
    // 对涉及的 shard 加锁。
    //
    // 为避免死锁：
    //
    // 一定按照 shard index 从小到大加锁。
    //
    pub fn del<I, S>(&self, keys: I) -> usize
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let keys: Vec<String> = keys
            .into_iter()
            .map(|key| key.as_ref().to_string())
            .collect();

        if keys.is_empty() {
            return 0;
        }

        //
        // 获取所有 shard
        //
        let mut shards: Vec<usize> = keys.iter().map(|key| Self::hash_key(key)).collect();

        shards.sort_unstable();
        shards.dedup();

        //
        // 按顺序加锁。
        //
        let mut guards = Vec::with_capacity(shards.len());

        for shard in &shards {
            guards.push(self.command_locks[*shard].lock().unwrap());
        }

        let mut count = 0;

        for key in &keys {
            if self.simple_cache.contains_key(key) {
                self.simple_cache.invalidate(key);
                count += 1;
            }
        }

        count
    }

    //
    // ========================================================
    // EXISTS
    // ========================================================
    //

    pub fn exists(&self, key: &str) -> bool {
        self.simple_cache.contains_key(key)
    }

    //
    // ========================================================
    // LEN
    // ========================================================
    //

    pub fn len(&self) -> u64 {
        self.simple_cache.iter().count() as u64
    }

    //
    // ========================================================
    // STRLEN
    // ========================================================
    //

    pub fn strlen(&self, key: &str) -> usize {
        self.get(key).map(|v| v.len()).unwrap_or(0)
    }

    //
    // ========================================================
    // INCR
    // ========================================================
    //

    pub fn incr(&self, key: &str) -> Result<i64, String> {
        self.incrby(key, 1)
    }

    //
    // ========================================================
    // INCRBY
    // ========================================================
    //

    pub fn incrby(&self, key: &str, amount: i64) -> Result<i64, String> {
        let lock = self.command_lock(key);
        let _guard = lock.lock().unwrap();

        let old = self.simple_cache.get(key);

        let (old_value, expires_at) = match old {
            Some(v) => {
                let number = v
                    .value
                    .parse::<i64>()
                    .map_err(|_| format!("value is not an integer: {}", v.value))?;

                (number, v.expires_at)
            }

            None => (0, None),
        };

        let new_value = old_value
            .checked_add(amount)
            .ok_or_else(|| "increment or decrement would overflow".to_string())?;

        //
        // 保留原 TTL。
        //
        self.simple_cache.insert(
            key.to_string(),
            ValueWithTtl {
                value: new_value.to_string(),
                expires_at,
            },
        );

        Ok(new_value)
    }

    //
    // ========================================================
    // INCRBYFLOAT
    // ========================================================
    //

    pub fn incrbyfloat(&self, key: &str, amount: f64) -> Result<f64, String> {
        let lock = self.command_lock(key);
        let _guard = lock.lock().unwrap();

        let old = self.simple_cache.get(key);

        let (old_value, expires_at) = match old {
            Some(v) => {
                let number = v
                    .value
                    .parse::<f64>()
                    .map_err(|_| format!("value is not a float: {}", v.value))?;

                (number, v.expires_at)
            }

            None => (0.0, None),
        };

        let new_value = old_value + amount;

        if !new_value.is_finite() {
            return Err("increment would produce a non-finite number".to_string());
        }

        self.simple_cache.insert(
            key.to_string(),
            ValueWithTtl {
                value: new_value.to_string(),
                expires_at,
            },
        );

        Ok(new_value)
    }

    //
    // ========================================================
    // 获取 / 创建 Hash Table
    // ========================================================
    //

    fn get_or_create_table(&self, table: &str) -> HashTable {
        let table_name = table.to_string();

        let value = self.table_cache.get_with(table_name, || ValueWithTtl {
            value: Arc::new(RwLock::new(HashMap::new())),
            expires_at: None,
        });

        value.value.clone()
    }

    //
    // ========================================================
    // HSET
    // ========================================================
    //

    pub fn hset(&self, table: &str, key: &str, value: &str) -> bool {
        //
        // table 本身的 RwLock。
        //
        let table_data = self.get_or_create_table(table);

        let mut map = table_data.write().unwrap();

        let is_new = !map.contains_key(key);

        map.insert(key.to_string(), value.to_string());

        is_new
    }

    //
    // ========================================================
    // HSETNX
    // ========================================================
    //

    pub fn hsetnx(&self, table: &str, key: &str, value: &str) -> bool {
        let table_data = self.get_or_create_table(table);

        let mut map = table_data.write().unwrap();

        if map.contains_key(key) {
            return false;
        }

        map.insert(key.to_string(), value.to_string());

        true
    }

    //
    // ========================================================
    // HGET
    // ========================================================
    //

    pub fn hget(&self, table: &str, key: &str) -> Option<String> {
        let value = self.table_cache.get(table)?;

        let map = value.value.read().unwrap();

        map.get(key).cloned()
    }

    //
    // ========================================================
    // HDEL
    // ========================================================
    //

    pub fn hdel(&self, table: &str, keys: &[&str]) -> usize {
        let value = match self.table_cache.get(table) {
            Some(v) => v,
            None => return 0,
        };

        let mut map = value.value.write().unwrap();

        let mut count = 0;

        for key in keys {
            if map.remove(*key).is_some() {
                count += 1;
            }
        }

        count
    }

    //
    // ========================================================
    // HEXISTS
    // ========================================================
    //

    pub fn hexists(&self, table: &str, key: &str) -> bool {
        let value = match self.table_cache.get(table) {
            Some(v) => v,
            None => return false,
        };

        let map = value.value.read().unwrap();

        map.contains_key(key)
    }

    //
    // ========================================================
    // HGETALL
    // ========================================================
    //

    pub fn hgetall(&self, table: &str) -> HashMap<String, String> {
        let value = match self.table_cache.get(table) {
            Some(v) => v,
            None => return HashMap::new(),
        };

        let map = value.value.read().unwrap();

        map.clone()
    }

    //
    // ========================================================
    // HKEYS
    // ========================================================
    //

    pub fn hkeys(&self, table: &str) -> Vec<String> {
        let value = match self.table_cache.get(table) {
            Some(v) => v,
            None => return Vec::new(),
        };

        let map = value.value.read().unwrap();

        map.keys().cloned().collect()
    }

    //
    // ========================================================
    // HVALS
    // ========================================================
    //

    pub fn hvals(&self, table: &str) -> Vec<String> {
        let value = match self.table_cache.get(table) {
            Some(v) => v,
            None => return Vec::new(),
        };

        let map = value.value.read().unwrap();

        map.values().cloned().collect()
    }

    //
    // ========================================================
    // HLEN
    // ========================================================
    //

    pub fn hlen(&self, table: &str) -> usize {
        let value = match self.table_cache.get(table) {
            Some(v) => v,
            None => return 0,
        };

        let map = value.value.read().unwrap();

        map.len()
    }

    //
    // ========================================================
    // HMGET
    // ========================================================
    //

    pub fn hmget(&self, table: &str, keys: &[&str]) -> Vec<Option<String>> {
        let value = match self.table_cache.get(table) {
            Some(v) => v,

            None => {
                return keys.iter().map(|_| None).collect();
            }
        };

        let map = value.value.read().unwrap();

        keys.iter().map(|key| map.get(*key).cloned()).collect()
    }

    //
    // ========================================================
    // HSTRLEN
    // ========================================================
    //

    pub fn hstrlen(&self, table: &str, key: &str) -> usize {
        self.hget(table, key).map(|v| v.len()).unwrap_or(0)
    }

    //
    // ========================================================
    // HINCR
    // ========================================================
    //

    pub fn hincr(&self, table: &str, key: &str) -> Result<i64, String> {
        self.hincrby(table, key, 1)
    }

    //
    // ========================================================
    // HINCRBY
    // ========================================================
    //

    pub fn hincrby(&self, table: &str, key: &str, amount: i64) -> Result<i64, String> {
        let table_data = self.get_or_create_table(table);

        let mut map = table_data.write().unwrap();

        let old_value = match map.get(key) {
            Some(value) => value
                .parse::<i64>()
                .map_err(|_| format!("hash value is not an integer: {}", value))?,

            None => 0,
        };

        let new_value = old_value
            .checked_add(amount)
            .ok_or_else(|| "increment would overflow".to_string())?;

        map.insert(key.to_string(), new_value.to_string());

        Ok(new_value)
    }

    //
    // ========================================================
    // HINCRBYFLOAT
    // ========================================================
    //

    pub fn hincrbyfloat(&self, table: &str, key: &str, amount: f64) -> Result<f64, String> {
        let table_data = self.get_or_create_table(table);

        let mut map = table_data.write().unwrap();

        let old_value = match map.get(key) {
            Some(value) => value
                .parse::<f64>()
                .map_err(|_| format!("hash value is not a float: {}", value))?,

            None => 0.0,
        };

        let new_value = old_value + amount;

        if !new_value.is_finite() {
            return Err("increment would produce a non-finite number".to_string());
        }

        map.insert(key.to_string(), new_value.to_string());

        Ok(new_value)
    }

    //
    // ========================================================
    // HSET_TTL
    // ========================================================
    //

    pub fn hset_ttl(&self, table: &str, ttl: Option<Duration>) -> bool {
        //
        // table 的生命周期操作需要和 HSET/HDEL_TABLE
        // 使用同一个 shard。
        //
        let lock = self.command_lock(table);
        let _guard = lock.lock().unwrap();

        let old = match self.table_cache.get(table) {
            Some(v) => v,
            None => return false,
        };

        self.table_cache
            .insert(table.to_string(), ValueWithTtl::new(old.value.clone(), ttl));

        true
    }

    //
    // ========================================================
    // HTTL
    // ========================================================
    //

    pub fn httl(&self, table: &str) -> i64 {
        match self.table_cache.get(table) {
            None => -2,

            Some(value) => match value.expires_at {
                None => -1,

                Some(expires_at) => expires_at
                    .saturating_duration_since(Instant::now())
                    .as_secs() as i64,
            },
        }
    }

    //
    // ========================================================
    // HDEL TABLE
    // ========================================================
    //

    pub fn hdel_table(&self, table: &str) -> bool {
        let lock = self.command_lock(table);
        let _guard = lock.lock().unwrap();

        if !self.table_cache.contains_key(table) {
            return false;
        }

        self.table_cache.invalidate(table);

        true
    }

    //
    // ========================================================
    // TABLE EXISTS
    // ========================================================
    //

    pub fn hexists_table(&self, table: &str) -> bool {
        self.table_cache.contains_key(table)
    }

    //
    // ========================================================
    // TABLE COUNT
    // ========================================================
    //

    pub fn table_count(&self) -> u64 {
        self.table_cache.iter().count() as u64
    }

    //
    // ========================================================
    // FIELD COUNT
    // ========================================================
    //

    pub fn hfield_count(&self, table: &str) -> usize {
        self.hlen(table)
    }
}
