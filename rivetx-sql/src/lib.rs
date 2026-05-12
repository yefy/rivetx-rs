pub mod conn;
pub mod create;
pub mod delete;
pub mod insert;
pub mod select;
pub mod sql_value;
pub mod update;
pub mod util;

pub mod create_tests;
#[cfg(test)]
mod create_test;
pub mod delete_tests;
#[cfg(test)]
mod delete_test;
pub mod insert_test;
pub mod insert_tests;
pub mod insert_tests2;
pub mod rivetx_sql_tests;
pub mod select_test;
pub mod select_tests;
pub mod update_test;
pub mod update_tests;
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
    fn to_values_discard_auto(&self) -> Vec<mysql_async::Value>;
    fn to_values(&self) -> Vec<mysql_async::Value>;
}
