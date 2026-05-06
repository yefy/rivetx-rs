use crate::arc_string::ArcString;
use mysql_common::value::convert::FromValue;
use mysql_common::value::Value;
use serde::de::{Deserialize, Deserializer};
use serde::ser::Serializer;
use serde::Serialize;
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

pub enum RivetxString {
    Owned(String),
    SharedStr(Arc<str>),
    SharedString(Arc<String>),
    ArcString(ArcString),
    Static(&'static str),
}

impl RivetxString {
    pub fn from_str(s: &str) -> Self {
        RivetxString::Owned(s.to_string())
    }
}

// impl From<&str> for RivetxString {
//     fn from(s: &str) -> Self {
//         RivetxString::Owned(s.to_string())
//     }
// }

impl From<String> for RivetxString {
    fn from(s: String) -> Self {
        RivetxString::Owned(s)
    }
}

impl From<Arc<str>> for RivetxString {
    fn from(s: Arc<str>) -> Self {
        RivetxString::SharedStr(s)
    }
}

impl From<Arc<String>> for RivetxString {
    fn from(s: Arc<String>) -> Self {
        RivetxString::SharedString(s)
    }
}

impl From<ArcString> for RivetxString {
    fn from(s: ArcString) -> Self {
        RivetxString::ArcString(s)
    }
}

impl From<&'static str> for RivetxString {
    fn from(s: &'static str) -> Self {
        RivetxString::Static(s)
    }
}

impl RivetxString {
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

    pub fn push_str(&mut self, string: &str) {
        *self = Self::from(format!("{}{}", self.as_str(), string));
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
        // 比较字典序
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
    #[inline]
    pub fn len(&self) -> usize {
        self.as_str().len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn to_string(&self) -> String {
        self.as_str().to_string()
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
            std::borrow::Cow::Borrowed(s) => Self::Owned(s.to_string()),
            std::borrow::Cow::Owned(s) => Self::Owned(s),
        }
    }
}
