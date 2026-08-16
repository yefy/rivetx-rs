pub mod backend;
pub mod conn;
pub mod create;
pub mod delete;
pub mod insert;
pub mod select;
pub mod sql_cell;
pub mod sql_value;
pub mod update;
pub mod util;

extern crate self as rivetx_sql;

#[cfg(all(test, feature = "native"))]
mod create_test;
#[cfg(feature = "native")]
pub mod create_tests;
#[cfg(all(test, feature = "native"))]
mod delete_test;
#[cfg(feature = "native")]
pub mod delete_tests;
#[cfg(feature = "native")]
pub mod insert_test;
#[cfg(feature = "native")]
pub mod insert_tests;
#[cfg(feature = "native")]
pub mod insert_tests2;
#[cfg(feature = "native")]
pub mod rivetx_sql_tests;
#[cfg(feature = "native")]
pub mod select_test;
#[cfg(feature = "native")]
pub mod select_tests;
#[cfg(feature = "native")]
pub mod update_test;
#[cfg(feature = "native")]
pub mod update_tests;
#[cfg(all(test, feature = "native"))]
mod util_test;
#[cfg(feature = "native")]
pub mod util_tests;

pub use rivetx_sql_derive::FromSqlRow;

use rivetx_core::rivetx_string::RivetxString;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct StructMeta {
    pub cols: Vec<RivetxString>,
    pub field_index: Vec<usize>,
    pub sql_field_names: Vec<RivetxString>,
    pub sql_types: Vec<RivetxString>,
    pub fixed_attrs: Vec<RivetxString>,

    pub discard_auto_cols: Vec<RivetxString>,
    pub discard_auto_field_index: Vec<usize>,

    pub auto_col_map: HashMap<RivetxString, bool>,
    pub primary: Option<RivetxString>,
    pub index_map: HashMap<RivetxString, RivetxString>,
    pub unique_map: HashMap<RivetxString, Vec<RivetxString>>,
}

pub trait FromSqlRow {
    fn get_struct_meta() -> StructMeta;
}

pub trait ToSqlValues {
    fn to_values_discard_auto(&self) -> Vec<crate::sql_cell::SqlCell>;
    fn to_values(&self) -> Vec<crate::sql_cell::SqlCell>;
}

pub use crate::backend::SqlBackend;
#[cfg(feature = "native")]
pub use crate::backend::MysqlBackend;
pub use crate::conn::RivetxSql;
pub use crate::sql_cell::{
    take_sql_cell, FromSqlCell, FromSqlCells, SqlCell, SqlExecResult, SqlValue,
};
