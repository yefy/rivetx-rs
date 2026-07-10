use std::borrow::Cow;
use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::sync::Arc;

use crate::arc_string::ArcString;
use crate::rivetx_string::RivetxString;

/// A unified string view that can borrow or own any Rivetx string type with minimal copying.
#[derive(Clone)]
pub enum RivetxStr<'a> {
    Ref(&'a str),
    Static(&'static str),
    RivetxStringRef(&'a RivetxString),
    ArcStringRef(&'a ArcString),
    RivetxString(RivetxString),
    ArcString(ArcString),
}

impl<'a> RivetxStr<'a> {
    pub fn from_str(s: &'a str) -> Self {
        if s.is_empty() {
            return RivetxStr::Static("");
        }
        RivetxStr::Ref(s)
    }

    #[inline]
    pub fn from_static(s: &'static str) -> RivetxStr<'static> {
        RivetxStr::Static(s)
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Ref(s) => s,
            Self::Static(s) => s,
            Self::RivetxStringRef(s) => s.as_str(),
            Self::ArcStringRef(s) => s.as_str(),
            Self::RivetxString(s) => s.as_str(),
            Self::ArcString(s) => s.as_str(),
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.as_str().len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clone into an owned [`RivetxString`]. Borrows are copied only when necessary.
    pub fn to_rivetx_string(&self) -> RivetxString {
        match self {
            Self::Ref(s) => RivetxString::from_str(s),
            Self::Static(s) => RivetxString::from(*s),
            Self::RivetxStringRef(s) => (*s).clone(),
            Self::ArcStringRef(s) => RivetxString::from((*s).clone()),
            Self::RivetxString(s) => s.clone(),
            Self::ArcString(s) => RivetxString::from(s.clone()),
        }
    }

    /// Clone into an owned [`ArcString`]. Shared backing storage is reused when possible.
    pub fn to_arc_string(&self) -> ArcString {
        match self {
            Self::Ref(s) => ArcString::from_str(s),
            Self::Static(s) => ArcString::from(*s),
            Self::RivetxStringRef(s) => match s {
                RivetxString::Owned(v) => ArcString::from(v.clone()),
                RivetxString::SharedStr(v) => ArcString::from(Arc::clone(v)),
                RivetxString::SharedString(v) => ArcString::from(Arc::clone(v)),
                RivetxString::ArcString(v) => v.clone(),
                RivetxString::Static(v) => ArcString::from(*v),
            },
            Self::ArcStringRef(s) => (*s).clone(),
            Self::RivetxString(s) => match s {
                RivetxString::Owned(v) => ArcString::from(v.clone()),
                RivetxString::SharedStr(v) => ArcString::from(Arc::clone(v)),
                RivetxString::SharedString(v) => ArcString::from(Arc::clone(v)),
                RivetxString::ArcString(v) => v.clone(),
                RivetxString::Static(v) => ArcString::from(*v),
            },
            Self::ArcString(s) => s.clone(),
        }
    }

    /// Consume and convert into an owned [`RivetxString`] without cloning when already owned.
    pub fn into_rivetx_string(self) -> RivetxString {
        match self {
            Self::RivetxString(s) => s,
            Self::ArcString(s) => RivetxString::from(s),
            other => other.to_rivetx_string(),
        }
    }

    /// Consume and convert into an owned [`ArcString`] without cloning when already owned.
    pub fn into_arc_string(self) -> ArcString {
        match self {
            Self::ArcString(s) => s,
            Self::RivetxString(s) => match s {
                RivetxString::ArcString(v) => v,
                other => RivetxStr::RivetxString(other).to_arc_string(),
            },
            other => other.to_arc_string(),
        }
    }
}

impl<'a> From<&'a str> for RivetxStr<'a> {
    #[inline]
    fn from(s: &'a str) -> Self {
        RivetxStr::from_str(s)
    }
}

impl<'a> From<&'a RivetxString> for RivetxStr<'a> {
    #[inline]
    fn from(s: &'a RivetxString) -> Self {
        if s.is_empty() {
            return RivetxStr::Static("");
        }
        RivetxStr::RivetxStringRef(s)
    }
}

impl<'a> From<&'a ArcString> for RivetxStr<'a> {
    #[inline]
    fn from(s: &'a ArcString) -> Self {
        if s.is_empty() {
            return RivetxStr::Static("");
        }
        RivetxStr::ArcStringRef(s)
    }
}

impl From<RivetxString> for RivetxStr<'static> {
    #[inline]
    fn from(s: RivetxString) -> Self {
        if s.is_empty() {
            return RivetxStr::Static("");
        }
        RivetxStr::RivetxString(s)
    }
}

impl From<ArcString> for RivetxStr<'static> {
    #[inline]
    fn from(s: ArcString) -> Self {
        if s.is_empty() {
            return RivetxStr::Static("");
        }
        RivetxStr::ArcString(s)
    }
}

impl From<String> for RivetxStr<'static> {
    #[inline]
    fn from(s: String) -> Self {
        RivetxStr::from(RivetxString::from(s))
    }
}

impl From<Arc<str>> for RivetxStr<'static> {
    #[inline]
    fn from(s: Arc<str>) -> Self {
        RivetxStr::from(RivetxString::from(s))
    }
}

impl From<Arc<String>> for RivetxStr<'static> {
    #[inline]
    fn from(s: Arc<String>) -> Self {
        RivetxStr::from(RivetxString::from(s))
    }
}

impl<'a> From<Cow<'a, str>> for RivetxStr<'a> {
    #[inline]
    fn from(c: Cow<'a, str>) -> Self {
        match c {
            Cow::Borrowed(s) => RivetxStr::Ref(s),
            Cow::Owned(s) => RivetxStr::from(RivetxString::from(s)),
        }
    }
}

impl<'a> Deref for RivetxStr<'a> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl<'a> fmt::Display for RivetxStr<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.as_str(), f)
    }
}

impl<'a> fmt::Debug for RivetxStr<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("RivetxStr").field(&self.as_str()).finish()
    }
}

impl<'a> PartialEq for RivetxStr<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl<'a> PartialEq<&str> for RivetxStr<'a> {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl<'a> Eq for RivetxStr<'a> {}

impl<'a> PartialOrd for RivetxStr<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> Ord for RivetxStr<'a> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl<'a> AsRef<str> for RivetxStr<'a> {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl<'a> AsRef<[u8]> for RivetxStr<'a> {
    fn as_ref(&self) -> &[u8] {
        self.as_str().as_bytes()
    }
}

impl<'a> std::borrow::Borrow<str> for RivetxStr<'a> {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl<'a> Hash for RivetxStr<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

impl<'a> Default for RivetxStr<'a> {
    fn default() -> Self {
        RivetxStr::Static("")
    }
}

impl From<RivetxStr<'_>> for RivetxString {
    fn from(s: RivetxStr<'_>) -> Self {
        s.into_rivetx_string()
    }
}

impl From<RivetxStr<'_>> for ArcString {
    fn from(s: RivetxStr<'_>) -> Self {
        s.into_arc_string()
    }
}
