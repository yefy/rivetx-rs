use crate::arc_string::ArcString;
use mysql_common::value::convert::FromValue;
use mysql_common::value::Value;
use serde::de::{Deserialize, Deserializer};
use serde::ser::Serializer;
use serde::Serialize;
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
use std::sync::Arc;

const DEFAULT_OWNED_STRING_MAX_SIZE: usize = 128;
const MIN_OWNED_STRING_MAX_SIZE: usize = 32;

static OWNED_STRING_MAX_SIZE: AtomicUsize = AtomicUsize::new(DEFAULT_OWNED_STRING_MAX_SIZE);

pub fn set_owned_string_max_size(max_size: usize) -> usize {
    let max_size = max_size.max(MIN_OWNED_STRING_MAX_SIZE);
    OWNED_STRING_MAX_SIZE.store(max_size, AtomicOrdering::Relaxed);
    max_size
}

pub fn owned_string_max_size() -> usize {
    OWNED_STRING_MAX_SIZE.load(AtomicOrdering::Relaxed)
}

pub enum RivetxString {
    Owned(String),
    SharedStr(Arc<str>),
    SharedString(Arc<String>),
    ArcString(ArcString),
    Static(&'static str),
}

impl RivetxString {
    pub fn to_arc_string(self) -> ArcString {
        match self {
            Self::Owned(s) => ArcString::from(s),
            Self::SharedStr(s) => ArcString::from(s),
            Self::SharedString(s) => ArcString::from(s),
            Self::ArcString(s) => s,
            Self::Static(s) => ArcString::from(s),
        }
    }
}

impl RivetxString {
    pub fn from_str(s: &str) -> Self {
        if s.is_empty() {
            return RivetxString::from("");
        }
        RivetxString::from(s.to_string())
    }
}

// impl From<&str> for RivetxString {
//     fn from(s: &str) -> Self {
//             if s.is_empty() {
//                 return RivetxString::from("");
//             }
//             RivetxString::from(s.to_string())
//     }
// }

impl From<String> for RivetxString {
    fn from(s: String) -> Self {
        if s.is_empty() {
            return RivetxString::from("");
        }
        if s.len() > owned_string_max_size() {
            RivetxString::from(Arc::new(s))
        } else {
            RivetxString::Owned(s)
        }
    }
}

impl From<Arc<str>> for RivetxString {
    fn from(s: Arc<str>) -> Self {
        if s.is_empty() {
            return RivetxString::from("");
        }
        RivetxString::SharedStr(s)
    }
}

impl From<Arc<String>> for RivetxString {
    fn from(s: Arc<String>) -> Self {
        if s.is_empty() {
            return RivetxString::from("");
        }
        RivetxString::SharedString(s)
    }
}

impl From<ArcString> for RivetxString {
    fn from(s: ArcString) -> Self {
        if s.is_empty() {
            return RivetxString::from("");
        }
        RivetxString::ArcString(s)
    }
}

impl From<&'static str> for RivetxString {
    fn from(s: &'static str) -> Self {
        RivetxString::Static(s)
    }
}

impl std::ops::Deref for RivetxString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl std::fmt::Display for RivetxString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::fmt::Debug for RivetxString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Owned(s) => f.debug_tuple("Owned").field(s).finish(),
            Self::SharedStr(s) => f.debug_tuple("SharedStr").field(s).finish(),
            Self::SharedString(s) => f.debug_tuple("SharedString").field(s).finish(),
            Self::ArcString(s) => f.debug_tuple("ArcString").field(s).finish(),
            Self::Static(s) => f.debug_tuple("Static").field(s).finish(),
        }
    }
}

impl PartialEq for RivetxString {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
    fn ne(&self, other: &Self) -> bool {
        PartialEq::ne(self.as_str(), other.as_str())
    }
}

impl PartialEq<&str> for RivetxString {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl Eq for RivetxString {}

impl PartialOrd for RivetxString {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RivetxString {
    fn cmp(&self, other: &Self) -> Ordering {
        // Compare lexicographic order
        self.as_str().cmp(other.as_str())
    }
}

impl Clone for RivetxString {
    fn clone(&self) -> Self {
        match self {
            Self::Owned(s) => Self::Owned(s.clone()),
            Self::SharedStr(s) => Self::SharedStr(Arc::clone(s)),
            Self::SharedString(s) => Self::SharedString(Arc::clone(s)),
            Self::ArcString(s) => Self::ArcString(s.clone()),
            Self::Static(s) => Self::Static(s),
        }
    }
}

impl RivetxString {
    pub fn new(str: String) -> Self {
        RivetxString::from(str)
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.as_str().len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        match self {
            RivetxString::Owned(s) => s.as_str(),
            RivetxString::SharedStr(s) => s,
            RivetxString::SharedString(s) => s.as_str(),
            RivetxString::ArcString(s) => s.as_str(),
            RivetxString::Static(s) => s,
        }
    }
    pub fn to_string(&self) -> String {
        self.as_str().to_string()
    }

    pub fn into_string(self) -> String {
        match self {
            RivetxString::Owned(s) => s,
            _ => self.as_str().to_string(),
        }
    }

    pub fn push_str(&mut self, string: &str) {
        match self {
            RivetxString::Owned(s) => s.push_str(string),
            _ => *self = Self::from(format!("{}{}", self.as_str(), string)),
        }
    }

    pub fn trim(&self) -> Self {
        let s = self.as_str();
        let trimmed = s.trim();
        if trimmed.len() == s.len() {
            return self.clone();
        }

        Self::from_str(trimmed)
    }
}

impl AsRef<[u8]> for RivetxString {
    fn as_ref(&self) -> &[u8] {
        self.as_str().as_bytes()
    }
}

impl AsRef<str> for RivetxString {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Default for RivetxString {
    #[inline]
    fn default() -> Self {
        RivetxString::from("")
    }
}

impl Hash for RivetxString {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

impl FromValue for RivetxString {
    type Intermediate = String;
}

impl From<RivetxString> for Value {
    fn from(arc_str: RivetxString) -> Value {
        Value::Bytes(arc_str.as_str().as_bytes().to_vec())
    }
}

impl Serialize for RivetxString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for RivetxString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(RivetxString::from(s))
    }
}

impl RivetxString {
    pub fn into_arc_string(self) -> Self {
        match self {
            Self::Owned(s) => Self::ArcString(ArcString::from(s)),
            _ => self,
        }
    }
}

impl RivetxString {
    pub fn clone_to_arc_string(&self) -> Self {
        match self {
            Self::Owned(s) => Self::ArcString(ArcString::from(s.clone())),
            _ => self.clone(),
        }
    }
}

impl std::borrow::Borrow<str> for RivetxString {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl<'a> From<std::borrow::Cow<'a, str>> for RivetxString {
    fn from(c: std::borrow::Cow<'a, str>) -> Self {
        match c {
            std::borrow::Cow::Borrowed(s) => Self::from(s.to_string()),
            std::borrow::Cow::Owned(s) => Self::from(s),
        }
    }
}
