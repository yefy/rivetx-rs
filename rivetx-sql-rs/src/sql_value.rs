use mysql_async::Value;
use rivetx_core_rs::arc_string::ArcString;
use std::sync::Arc;

#[derive(Clone)]
pub enum SqlValue {
    RealValue(mysql_async::Value),
    SharedString(ArcString),
}

impl From<String> for SqlValue {
    fn from(s: String) -> Self {
        SqlValue::SharedString(ArcString::from(s))
    }
}

impl From<&str> for SqlValue {
    fn from(s: &str) -> Self {
        SqlValue::SharedString(ArcString::from_str(s))
    }
}

impl From<Arc<String>> for SqlValue {
    fn from(s: Arc<String>) -> Self {
        SqlValue::SharedString(ArcString::from(s))
    }
}

impl From<Arc<str>> for SqlValue {
    fn from(s: Arc<str>) -> Self {
        SqlValue::SharedString(ArcString::from(s))
    }
}

impl From<ArcString> for SqlValue {
    fn from(s: ArcString) -> Self {
        SqlValue::SharedString(s)
    }
}

impl From<mysql_async::Value> for SqlValue {
    fn from(v: mysql_async::Value) -> Self {
        SqlValue::RealValue(v)
    }
}

macro_rules! impl_real_value {
    ($($t:ty),*) => {
        $(
            impl From<$t> for SqlValue {
                fn from(v: $t) -> Self {
                    SqlValue::RealValue(mysql_async::Value::from(v))
                }
            }
        )*
    };
}

impl_real_value!(i8, i16, i32, i64, u8, u16, u32, u64, f32, f64, bool);

impl From<SqlValue> for Value {
    fn from(rv: SqlValue) -> Self {
        match rv {
            SqlValue::RealValue(v) => v,
            SqlValue::SharedString(s) => Value::Bytes(s.as_bytes().to_vec()),
        }
    }
}

impl std::fmt::Display for SqlValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RealValue(s) => write!(f, "{:?}", s),
            Self::SharedString(s) => write!(f, "{}", s),
        }
    }
}

impl std::fmt::Debug for SqlValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RealValue(s) => f.debug_tuple("RealValue").field(s).finish(),
            Self::SharedString(s) => f.debug_tuple("SharedString").field(s).finish(),
        }
    }
}
