use anyhow::anyhow;
use rivetx_core::arc_string::ArcString;
use rivetx_core::rivetx_string::RivetxString;
use std::sync::Arc;

#[derive(Clone)]
pub enum SqlCell {
    Null,
    Bool(bool),
    I64(i64),
    U64(u64),
    F64(f64),
    Str(RivetxString),
    Bytes(Vec<u8>),
    DateTime(chrono::NaiveDateTime),
}

pub type SqlValue = SqlCell;

#[derive(Clone, Debug, Default)]
pub struct SqlExecResult {
    pub cols: Vec<RivetxString>,
    pub rows: Vec<Vec<SqlCell>>,
    pub affected: u64,
    pub last_insert_id: u64,
}

pub trait FromSqlCell: Sized {
    fn from_sql_cell(cell: SqlCell) -> anyhow::Result<Self>;
}

pub trait FromSqlCells: Sized {
    fn from_sql_cells(cols: &[RivetxString], cells: &[SqlCell]) -> anyhow::Result<Self>;
}

pub fn take_sql_cell(
    cols: &[RivetxString],
    cells: &[SqlCell],
    name: &str,
) -> anyhow::Result<SqlCell> {
    for (i, col) in cols.iter().enumerate() {
        if col.as_str().eq_ignore_ascii_case(name) {
            return cells
                .get(i)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("missing cell for column {}", name));
        }
    }
    Err(anyhow::anyhow!("column {} not found in {:?}", name, cols))
}

impl SqlCell {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            SqlCell::Str(s) => Some(s.as_str()),
            _ => None,
        }
    }
}

impl From<String> for SqlCell {
    fn from(s: String) -> Self {
        SqlCell::Str(RivetxString::from(s))
    }
}

impl From<&str> for SqlCell {
    fn from(s: &str) -> Self {
        SqlCell::Str(RivetxString::from_str(s))
    }
}

impl From<Arc<String>> for SqlCell {
    fn from(s: Arc<String>) -> Self {
        SqlCell::Str(RivetxString::from(s))
    }
}

impl From<Arc<str>> for SqlCell {
    fn from(s: Arc<str>) -> Self {
        SqlCell::Str(RivetxString::from(s))
    }
}

impl From<ArcString> for SqlCell {
    fn from(s: ArcString) -> Self {
        SqlCell::Str(RivetxString::from(s))
    }
}

impl From<RivetxString> for SqlCell {
    fn from(s: RivetxString) -> Self {
        SqlCell::Str(s)
    }
}

impl From<chrono::NaiveDateTime> for SqlCell {
    fn from(v: chrono::NaiveDateTime) -> Self {
        SqlCell::DateTime(v)
    }
}

macro_rules! impl_from_int {
    ($($t:ty => $var:ident),*) => {
        $(
            impl From<$t> for SqlCell {
                fn from(v: $t) -> Self {
                    SqlCell::$var(v as _)
                }
            }
        )*
    };
}

impl_from_int!(
    i8 => I64,
    i16 => I64,
    i32 => I64,
    i64 => I64,
    u8 => U64,
    u16 => U64,
    u32 => U64,
    u64 => U64
);

impl From<f32> for SqlCell {
    fn from(v: f32) -> Self {
        SqlCell::F64(v as f64)
    }
}

impl From<f64> for SqlCell {
    fn from(v: f64) -> Self {
        SqlCell::F64(v)
    }
}

impl From<bool> for SqlCell {
    fn from(v: bool) -> Self {
        SqlCell::Bool(v)
    }
}

#[cfg(feature = "native")]
impl From<mysql_async::Value> for SqlCell {
    fn from(v: mysql_async::Value) -> Self {
        mysql_value_to_cell(v)
    }
}

#[cfg(feature = "native")]
fn mysql_value_to_cell(v: mysql_async::Value) -> SqlCell {
    use mysql_async::Value;
    match v {
        Value::NULL => SqlCell::Null,
        Value::Bytes(b) => match String::from_utf8(b.clone()) {
            Ok(s) => SqlCell::Str(RivetxString::from(s)),
            Err(e) => SqlCell::Bytes(e.into_bytes()),
        },
        Value::Int(i) => SqlCell::I64(i),
        Value::UInt(u) => SqlCell::U64(u),
        Value::Float(f) => SqlCell::F64(f as f64),
        Value::Double(f) => SqlCell::F64(f),
        Value::Date(y, m, d, h, min, s, micro) => {
            let dt = chrono::NaiveDate::from_ymd_opt(y as i32, m as u32, d as u32)
                .and_then(|date| date.and_hms_micro_opt(h as u32, min as u32, s as u32, micro));
            match dt {
                Some(dt) => SqlCell::DateTime(dt),
                None => SqlCell::Null,
            }
        }
        Value::Time(neg, days, h, min, s, micro) => {
            let mut secs = days as i64 * 86400 + h as i64 * 3600 + min as i64 * 60 + s as i64;
            if neg {
                secs = -secs;
            }
            SqlCell::Str(format!("time:{}:{}", secs, micro).into())
        }
    }
}

#[cfg(feature = "native")]
impl From<SqlCell> for mysql_async::Value {
    fn from(cell: SqlCell) -> Self {
        use mysql_async::Value;
        match cell {
            SqlCell::Null => Value::NULL,
            SqlCell::Bool(v) => Value::Int(if v { 1 } else { 0 }),
            SqlCell::I64(v) => Value::Int(v),
            SqlCell::U64(v) => Value::UInt(v),
            SqlCell::F64(v) => Value::Double(v),
            SqlCell::Str(s) => Value::Bytes(s.as_bytes().to_vec()),
            SqlCell::Bytes(b) => Value::Bytes(b),
            SqlCell::DateTime(dt) => Value::from(dt),
        }
    }
}

impl FromSqlCell for SqlCell {
    fn from_sql_cell(cell: SqlCell) -> anyhow::Result<Self> {
        Ok(cell)
    }
}

impl FromSqlCell for String {
    fn from_sql_cell(cell: SqlCell) -> anyhow::Result<Self> {
        match cell {
            SqlCell::Str(s) => Ok(s.into_string()),
            SqlCell::Bytes(b) => String::from_utf8(b).map_err(|e| anyhow!("utf8:{}", e)),
            SqlCell::Null => Ok(String::new()),
            other => Err(anyhow!("cannot convert {:?} to String", other)),
        }
    }
}

impl FromSqlCell for RivetxString {
    fn from_sql_cell(cell: SqlCell) -> anyhow::Result<Self> {
        match cell {
            SqlCell::Str(s) => Ok(s),
            other => Ok(RivetxString::from(String::from_sql_cell(other)?)),
        }
    }
}

impl FromSqlCell for bool {
    fn from_sql_cell(cell: SqlCell) -> anyhow::Result<Self> {
        match cell {
            SqlCell::Bool(v) => Ok(v),
            SqlCell::I64(v) => Ok(v != 0),
            SqlCell::U64(v) => Ok(v != 0),
            SqlCell::Null => Ok(false),
            other => Err(anyhow!("cannot convert {:?} to bool", other)),
        }
    }
}

fn parse_naive_datetime(s: &str) -> anyhow::Result<chrono::NaiveDateTime> {
    const FMTS: &[&str] = &[
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%d",
    ];
    for fmt in FMTS {
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, fmt) {
            return Ok(dt);
        }
        if *fmt == "%Y-%m-%d" {
            if let Ok(d) = chrono::NaiveDate::parse_from_str(s, fmt) {
                return Ok(d.and_hms_opt(0, 0, 0).unwrap());
            }
        }
    }
    Err(anyhow!("cannot parse NaiveDateTime from {}", s))
}

impl FromSqlCell for chrono::NaiveDateTime {
    fn from_sql_cell(cell: SqlCell) -> anyhow::Result<Self> {
        match cell {
            SqlCell::DateTime(v) => Ok(v),
            SqlCell::Str(s) => parse_naive_datetime(s.as_str()),
            SqlCell::Bytes(b) => {
                parse_naive_datetime(std::str::from_utf8(&b).map_err(|e| anyhow!("utf8:{}", e))?)
            }
            other => Err(anyhow!("cannot convert {:?} to NaiveDateTime", other)),
        }
    }
}

impl<T: FromSqlCell> FromSqlCell for Option<T> {
    fn from_sql_cell(cell: SqlCell) -> anyhow::Result<Self> {
        match cell {
            SqlCell::Null => Ok(None),
            other => T::from_sql_cell(other).map(Some),
        }
    }
}

macro_rules! impl_from_sql_int {
    ($t:ty) => {
        impl FromSqlCell for $t {
            fn from_sql_cell(cell: SqlCell) -> anyhow::Result<Self> {
                match cell {
                    SqlCell::I64(v) => <$t>::try_from(v)
                        .map_err(|_| anyhow!("i64 {} does not fit {}", v, stringify!($t))),
                    SqlCell::U64(v) => <$t>::try_from(v)
                        .map_err(|_| anyhow!("u64 {} does not fit {}", v, stringify!($t))),
                    SqlCell::Bool(v) => Ok(if v { 1 as $t } else { 0 as $t }),
                    SqlCell::Null => Ok(0 as $t),
                    other => Err(anyhow!("cannot convert {:?} to {}", other, stringify!($t))),
                }
            }
        }
    };
}

impl_from_sql_int!(i8);
impl_from_sql_int!(i16);
impl_from_sql_int!(i32);
impl_from_sql_int!(i64);
impl_from_sql_int!(u8);
impl_from_sql_int!(u16);
impl_from_sql_int!(u32);
impl_from_sql_int!(u64);

impl std::fmt::Display for SqlCell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SqlCell::Null => write!(f, "NULL"),
            SqlCell::Bool(v) => write!(f, "{}", v),
            SqlCell::I64(v) => write!(f, "{}", v),
            SqlCell::U64(v) => write!(f, "{}", v),
            SqlCell::F64(v) => write!(f, "{}", v),
            SqlCell::Str(s) => write!(f, "{}", s),
            SqlCell::Bytes(b) => write!(f, "{:?}", b),
            SqlCell::DateTime(v) => write!(f, "{}", v),
        }
    }
}

impl std::fmt::Debug for SqlCell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SqlCell::Null => write!(f, "Null"),
            SqlCell::Bool(v) => f.debug_tuple("Bool").field(v).finish(),
            SqlCell::I64(v) => f.debug_tuple("I64").field(v).finish(),
            SqlCell::U64(v) => f.debug_tuple("U64").field(v).finish(),
            SqlCell::F64(v) => f.debug_tuple("F64").field(v).finish(),
            SqlCell::Str(s) => f.debug_tuple("Str").field(s).finish(),
            SqlCell::Bytes(b) => f.debug_tuple("Bytes").field(b).finish(),
            SqlCell::DateTime(v) => f.debug_tuple("DateTime").field(v).finish(),
        }
    }
}
