use crate::rivetx_string::RivetxString;
use core::ops::RangeBounds;
use mysql_common::value::convert::FromValue;
use mysql_common::value::Value;
use serde::de::{Deserialize, Deserializer};
use serde::ser::Serializer;
use serde::Serialize;
use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::ops::Range;
use std::sync::Arc;

enum ArcStringValue {
    SharedStr(Arc<str>),
    SharedString(Arc<String>),
    Static(&'static str),
}

impl ArcStringValue {
    #[inline]
    pub fn as_str(&self) -> &str {
        match self {
            ArcStringValue::SharedStr(s) => s,
            ArcStringValue::SharedString(s) => s.as_str(),
            ArcStringValue::Static(s) => s,
        }
    }
}

impl Deref for ArcStringValue {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl Clone for ArcStringValue {
    fn clone(&self) -> Self {
        match self {
            Self::SharedStr(s) => Self::SharedStr(Arc::clone(s)),
            Self::SharedString(s) => Self::SharedString(Arc::clone(s)),
            Self::Static(s) => Self::Static(s),
        }
    }
}

//#[derive(Clone)]
pub struct ArcString {
    data: ArcStringValue,
    range: Range<usize>,
}

impl From<RivetxString> for ArcString {
    fn from(s: RivetxString) -> Self {
        s.to_arc_string()
    }
}

impl ArcString {
    pub fn from_str(s: &str) -> Self {
        if s.is_empty() {
            return ArcString::from("");
        }
        ArcString::from(s.to_string())
    }
}

// impl From<&str> for ArcString {
//     #[inline]
//     fn from(s: &str) -> Self {
//         if s.is_empty() {
//             return ArcString::from("");
//         }
//         Self::new(s.to_owned())
//     }
// }

impl From<String> for ArcString {
    #[inline]
    fn from(s: String) -> Self {
        if s.is_empty() {
            return ArcString::from("");
        }
        let len = s.len();
        Self {
            data: ArcStringValue::SharedString(Arc::new(s)),
            range: Range { start: 0, end: len },
        }
    }
}

impl From<Arc<str>> for ArcString {
    fn from(s: Arc<str>) -> Self {
        if s.is_empty() {
            return ArcString::from("");
        }
        let len = s.len();
        Self {
            data: ArcStringValue::SharedStr(s),
            range: Range { start: 0, end: len },
        }
    }
}

impl From<Arc<String>> for ArcString {
    fn from(s: Arc<String>) -> Self {
        if s.is_empty() {
            return ArcString::from("");
        }
        let len = s.len();
        Self {
            data: ArcStringValue::SharedString(s),
            range: Range { start: 0, end: len },
        }
    }
}

impl From<&'static str> for ArcString {
    fn from(s: &'static str) -> Self {
        let len = s.len();
        Self {
            data: ArcStringValue::Static(s),
            range: Range { start: 0, end: len },
        }
    }
}

impl Deref for ArcString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl fmt::Display for ArcString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.as_str(), f)
    }
}

impl fmt::Debug for ArcString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ArcString").field(&self.as_str()).finish()
    }
}

impl PartialEq for ArcString {
    fn eq(&self, other: &ArcString) -> bool {
        PartialEq::eq(&self[..], &other[..])
    }
    fn ne(&self, other: &ArcString) -> bool {
        PartialEq::ne(&self[..], &other[..])
    }
}

impl PartialEq<&str> for ArcString {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl Eq for ArcString {}

impl PartialOrd for ArcString {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for ArcString {
    fn cmp(&self, other: &Self) -> Ordering {
        // Compare lexicographic order
        self.as_str().cmp(other.as_str())
    }
}

impl Clone for ArcString {
    fn clone(&self) -> Self {
        match &self.data {
            ArcStringValue::SharedStr(s) => Self {
                data: ArcStringValue::SharedStr(Arc::clone(s)),
                range: self.range.clone(),
            },
            ArcStringValue::SharedString(s) => Self {
                data: ArcStringValue::SharedString(Arc::clone(s)),
                range: self.range.clone(),
            },
            ArcStringValue::Static(s) => Self {
                data: ArcStringValue::Static(s),
                range: self.range.clone(),
            },
        }
    }
}

impl ArcString {
    pub fn new(str: String) -> Self {
        ArcString::from(str)
    }

    pub fn len(&self) -> usize {
        self.range.end - self.range.start
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn as_str(&self) -> &str {
        &self.data[self.range.start..self.range.end]
    }

    pub fn to_string(&self) -> String {
        self.as_str().to_string()
    }

    pub fn push_str(&mut self, string: &str) {
        *self = Self::from(format!("{}{}", self.as_str(), string));
    }

    pub fn trim(&self) -> Self {
        let s = self.as_str();
        let trimmed = s.trim();
        if trimmed.len() == s.len() {
            return self.clone();
        }

        let start_offset = trimmed.as_ptr() as usize - s.as_ptr() as usize;
        self.slice(start_offset..start_offset + trimmed.len())
    }

    pub fn slice(&self, range: impl RangeBounds<usize>) -> Self {
        use core::ops::Bound;

        let len = self.len();

        let begin = match range.start_bound() {
            Bound::Included(&n) => n,
            Bound::Excluded(&n) => n + 1,
            Bound::Unbounded => 0,
        };

        let end = match range.end_bound() {
            Bound::Included(&n) => n.checked_add(1).expect("out of range"),
            Bound::Excluded(&n) => n,
            Bound::Unbounded => len,
        };

        assert!(
            begin <= end,
            "range start must not be greater than end: {:?} <= {:?}",
            begin,
            end,
        );
        assert!(
            end <= len,
            "range end out of bounds: {:?} <= {:?}",
            end,
            len,
        );

        if begin == end {
            return Self::default();
        }

        let diff_start = begin - 0;
        let diff_end = len - end;

        Self {
            data: self.data.clone(),
            range: Range {
                start: self.range.start + diff_start,
                end: self.range.end - diff_end,
            },
        }
    }

    pub fn split(&self, delimiter: &str) -> Vec<Self> {
        let s = self.as_str();
        let base_ptr = s.as_ptr() as usize;

        s.split(delimiter)
            .map(|part| {
                // Compute the offset of the current substring relative to self.as_str() start
                let part_ptr = part.as_ptr() as usize;
                let offset = part_ptr - base_ptr;

                // Slice based on the original self.range
                // New start is original start plus the substring offset in the current view
                let new_start = self.range.start + offset;
                let new_end = new_start + part.len();

                Self {
                    data: self.data.clone(),
                    range: Range {
                        start: new_start,
                        end: new_end,
                    },
                }
            })
            .collect()
    }
}

impl AsRef<[u8]> for ArcString {
    fn as_ref(&self) -> &[u8] {
        self.as_str().as_bytes()
    }
}

impl AsRef<str> for ArcString {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Default for ArcString {
    #[inline]
    fn default() -> Self {
        return ArcString::from("");
    }
}

impl Hash for ArcString {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

impl FromValue for ArcString {
    type Intermediate = String;

    // fn from_value_opt(value: Value) -> Result<Self, FromValueError> {
    //     match value {
    //         Value::Bytes(bytes) => {
    //             let string = String::from_utf8(bytes)
    //                 .map_err(|e| FromValueError(Value::Bytes(e.into_bytes())))?;
    //             Ok(ArcString::from(string))
    //         }
    //         _ => Err( FromValueError(Value::Bytes("Expected String or Bytes".as_bytes().to_vec()))),
    //     }
    // }
    //
    // fn get_intermediate(value: Value) -> Result<Self::Intermediate, FromValueError> {
    //     Self::Intermediate::try_from(value)
    // }
}

impl From<ArcString> for Value {
    fn from(arc_str: ArcString) -> Value {
        Value::Bytes(arc_str.as_str().as_bytes().to_vec()) // Convert ArcString to byte representation
    }
}

impl Serialize for ArcString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ArcString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(ArcString::new(s))
    }
}

impl std::borrow::Borrow<str> for ArcString {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl<'a> From<std::borrow::Cow<'a, str>> for ArcString {
    fn from(c: std::borrow::Cow<'a, str>) -> Self {
        match c {
            std::borrow::Cow::Borrowed(s) => Self::from_str(s),
            std::borrow::Cow::Owned(s) => Self::from(s),
        }
    }
}
