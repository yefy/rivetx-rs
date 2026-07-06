use chrono::NaiveDateTime;
use mysql_async::Value;
use rivetx_sql::sql_value::SqlValue;
use rivetx_sql::FromSqlRow;
use rivetx_sql::StructMeta;
use rivetx_sql::ToSqlValues;

#[derive(Default, Debug, Clone, PartialEq, FromSqlRow)]
pub struct TestData {
    #[attr(auto, primary)]
    #[db = "id"]
    pub id: u64,

    #[attr(unique = "u_td_ik", unique = "u_td_in")]
    #[db = "index_col"]
    pub index: i32,

    #[db = "key_col"]
    #[attr(unique = "u_td_ik")]
    #[size = "64"]
    pub key: String,

    #[db = "name_id"]
    #[attr(unique = "u_td_in")]
    pub name_id: i32,

    #[db = "name_index"]
    #[attr(index = "i_td_name_index")]
    pub name_index: i32,

    #[db = "curr_time"]
    pub curr_time: NaiveDateTime,

    #[db = "created_at"]
    #[attr("DEFAULT CURRENT_TIMESTAMP")]
    pub created_at: NaiveDateTime,

    #[db = "updated_at"]
    #[attr("DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP")]
    pub updated_at: NaiveDateTime,
}

impl rivetx_sql::select::OrderFieldSelectValue for TestData {
    fn order_field_select_value(&self) -> SqlValue {
        Value::from(self.id).into()
    }
}
