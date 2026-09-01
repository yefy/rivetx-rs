use crate::sql_value::SqlValue;
use rivetx_core::rivetx_string::RivetxString;
use std::time::Duration;

pub const BATCH_SIZE: usize = 1024;
pub const TIMEOUT: Duration = Duration::from_secs(120);

pub async fn sleep_duration(d: Duration) {
    #[cfg(feature = "native")]
    tokio::time::sleep(d).await;
    #[cfg(not(feature = "native"))]
    {
        let _ = d;
    }
}

#[derive(Clone)]
pub struct QueryCond {
    pub fixed_cols: Vec<RivetxString>,
    pub fixed_vals: Vec<SqlValue>,
    pub in_cols: Vec<RivetxString>,
    pub in_vals: Vec<Vec<SqlValue>>,
    pub in_batch_size: usize,
}

/// A value row for [`QueryCond::set_in_rows`]: a tuple of mixed types, or a
/// same-type array, whose length matches the column array.
pub trait IntoSqlRow<const N: usize> {
    fn into_sql_row(self) -> [SqlValue; N];
}

impl<V, const N: usize> IntoSqlRow<N> for [V; N]
where
    V: Into<SqlValue>,
{
    fn into_sql_row(self) -> [SqlValue; N] {
        self.map(Into::into)
    }
}

macro_rules! impl_into_sql_row_for_tuple {
    ($n:literal; $($i:tt : $T:ident),+ $(,)?) => {
        impl<$($T),+> IntoSqlRow<$n> for ($($T,)+)
        where
            $($T: Into<SqlValue>,)+
        {
            fn into_sql_row(self) -> [SqlValue; $n] {
                [$(self.$i.into()),+]
            }
        }
    };
}

impl_into_sql_row_for_tuple!(1; 0: A);
impl_into_sql_row_for_tuple!(2; 0: A, 1: B);
impl_into_sql_row_for_tuple!(3; 0: A, 1: B, 2: C);
impl_into_sql_row_for_tuple!(4; 0: A, 1: B, 2: C, 3: D);
impl_into_sql_row_for_tuple!(5; 0: A, 1: B, 2: C, 3: D, 4: E);
impl_into_sql_row_for_tuple!(6; 0: A, 1: B, 2: C, 3: D, 4: E, 5: F);
impl_into_sql_row_for_tuple!(7; 0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G);
impl_into_sql_row_for_tuple!(8; 0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H);

impl QueryCond {
    pub fn new() -> Self {
        Self {
            fixed_cols: Vec::new(),
            fixed_vals: Vec::new(),
            in_cols: Vec::new(),
            in_vals: Vec::new(),
            in_batch_size: 0,
        }
    }

    /// `col IN (v1, v2, v3)` — one column, a flat list of values.
    pub fn set_in<C, V, I>(&mut self, col: C, vals: I)
    where
        C: Into<RivetxString>,
        I: IntoIterator<Item = V>,
        V: Into<SqlValue>,
    {
        self.in_cols = vec![col.into()];
        self.in_vals = vals.into_iter().map(|v| vec![v.into()]).collect();
    }

    /// `(c1, c2, ...) IN ((v1, v2, ...), ...)` — column count is `N`.
    pub fn set_in_rows<C, R, I, const N: usize>(&mut self, cols: [C; N], rows: I)
    where
        C: Into<RivetxString>,
        I: IntoIterator<Item = R>,
        R: IntoSqlRow<N>,
    {
        let cols = cols.map(Into::into);
        self.in_cols = cols.to_vec();
        self.in_vals = rows
            .into_iter()
            .map(|row| row.into_sql_row().to_vec())
            .collect();
    }
}

impl Default for QueryCond {
    fn default() -> Self {
        Self::new()
    }
}

pub struct QueryStruct<F, I> {
    pub fixed: Option<F>,
    pub in_vals: Vec<I>,
}

impl<F, I> Default for QueryStruct<F, I> {
    fn default() -> Self {
        Self {
            fixed: None,
            in_vals: Vec::new(),
        }
    }
}

pub fn to_snake_case(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 2);
    for (i, r) in s.chars().enumerate() {
        if i > 0 && r.is_uppercase() {
            if let Some(last) = result.chars().last() {
                if last != '_' {
                    result.push('_');
                }
            }
        }
        result.push(r.to_lowercase().next().unwrap());
    }
    result
}

pub struct EstimateJoin<'a> {
    pub parts: &'a [RivetxString],
    pub sep: &'a str,
}

fn estimate_join_len(data: EstimateJoin) -> usize {
    if data.parts.is_empty() {
        return 0;
    }
    let total_parts_len: usize = data.parts.iter().map(|p| p.len()).sum();
    let total_sep_len = (data.parts.len() - 1) * data.sep.len();
    total_parts_len + total_sep_len
}

#[allow(clippy::too_many_arguments)]
pub fn build_query(
    sqls: &[&str],
    table: &str,
    join: &str,
    fixed_conds: &[RivetxString],
    cond: &str,
    in_cols: &[RivetxString],
    tuples: &[RivetxString],
    order: &str,
    limit: &str,
) -> String {
    let mut est_len = 128 + table.len() + join.len() + cond.len() + limit.len() + order.len();
    est_len += sqls.iter().map(|s| s.len() + 1).sum::<usize>();
    est_len += estimate_join_len(EstimateJoin {
        parts: fixed_conds,
        sep: " AND ",
    });
    est_len += estimate_join_len(EstimateJoin {
        parts: in_cols,
        sep: ", ",
    });
    est_len += estimate_join_len(EstimateJoin {
        parts: tuples,
        sep: ",",
    });

    let mut b = String::with_capacity(est_len);

    for s in sqls {
        b.push_str(s);
        b.push(' ');
    }
    b.push_str(table);
    b.push(' ');
    b.push_str(join);
    b.push_str(" WHERE");

    let mut is_first_add = true;
    let mut write_and = |builder: &mut String| {
        if is_first_add {
            is_first_add = false;
            builder.push(' ');
        } else {
            builder.push_str(" AND ");
        }
    };

    if !fixed_conds.is_empty() {
        write_and(&mut b);
        for (i, c) in fixed_conds.iter().enumerate() {
            if i > 0 {
                b.push_str(" AND ");
            }
            b.push_str(c);
        }
    }

    if !cond.is_empty() {
        write_and(&mut b);
        b.push_str(cond);
    }

    if !tuples.is_empty() {
        write_and(&mut b);
        b.push('(');
        for (i, c) in in_cols.iter().enumerate() {
            if i > 0 {
                b.push_str(", ");
            }
            b.push_str(c);
        }
        b.push_str(") IN (");
        for (i, t) in tuples.iter().enumerate() {
            if i > 0 {
                b.push(',');
            }
            b.push_str(t);
        }
        b.push(')');
    }

    if !order.is_empty() {
        b.push(' ');
        b.push_str(order);
    }

    if !limit.is_empty() {
        b.push(' ');
        b.push_str(limit);
    }

    b
}

#[cfg(test)]
mod in_api_tests {
    use super::{IntoSqlRow, QueryCond};
    use crate::sql_value::SqlValue;

    #[test]
    fn set_in_wraps_each_value_as_its_own_row() {
        let mut cond = QueryCond::new();
        cond.set_in("aaa", ["1", "2", "3"]);

        assert_eq!(cond.in_cols.len(), 1);
        assert_eq!(cond.in_cols[0], "aaa");
        assert_eq!(cond.in_vals.len(), 3);
        for (i, expected) in ["1", "2", "3"].iter().enumerate() {
            assert_eq!(cond.in_vals[i].len(), 1);
            assert_eq!(cond.in_vals[i][0].as_str(), Some(*expected));
        }
    }

    #[test]
    fn set_in_accepts_vec_and_empty_iter() {
        let mut cond = QueryCond::new();
        cond.set_in("id", vec![1i64, 2, 3]);
        assert_eq!(cond.in_vals.len(), 3);
        assert_eq!(cond.in_vals[0].len(), 1);

        cond.set_in("id", Vec::<i64>::new());
        assert!(cond.in_vals.is_empty());
        assert_eq!(cond.in_cols[0], "id");
    }

    #[test]
    fn set_in_rows_mixed_tuple() {
        let mut cond = QueryCond::new();
        cond.set_in_rows(["id", "name"], vec![(1i64, "a"), (2i64, "b")]);

        assert_eq!(cond.in_cols.len(), 2);
        assert_eq!(cond.in_cols[0], "id");
        assert_eq!(cond.in_cols[1], "name");
        assert_eq!(cond.in_vals.len(), 2);
        assert_eq!(cond.in_vals[0].len(), 2);
        match &cond.in_vals[0][0] {
            SqlValue::I64(v) => assert_eq!(*v, 1),
            other => panic!("expected I64, got {:?}", other),
        }
        assert_eq!(cond.in_vals[0][1].as_str(), Some("a"));
        match &cond.in_vals[1][0] {
            SqlValue::I64(v) => assert_eq!(*v, 2),
            other => panic!("expected I64, got {:?}", other),
        }
        assert_eq!(cond.in_vals[1][1].as_str(), Some("b"));
    }

    #[test]
    fn set_in_rows_same_type_array() {
        let mut cond = QueryCond::new();
        cond.set_in_rows(["a", "b"], vec![[1i32, 2], [3, 4]]);

        assert_eq!(cond.in_vals.len(), 2);
        assert_eq!(cond.in_vals[0].len(), 2);
        match (&cond.in_vals[0][0], &cond.in_vals[0][1]) {
            (SqlValue::I64(a), SqlValue::I64(b)) => {
                assert_eq!(*a, 1);
                assert_eq!(*b, 2);
            }
            other => panic!("expected I64 pair, got {:?}", other),
        }
    }

    #[test]
    fn into_sql_row_tuple_arity() {
        let row1: [SqlValue; 1] = ("x",).into_sql_row();
        assert_eq!(row1[0].as_str(), Some("x"));

        let row3: [SqlValue; 3] = (1i64, "n", true).into_sql_row();
        match &row3[0] {
            SqlValue::I64(v) => assert_eq!(*v, 1),
            other => panic!("expected I64, got {:?}", other),
        }
        assert_eq!(row3[1].as_str(), Some("n"));
        match &row3[2] {
            SqlValue::Bool(v) => assert!(*v),
            other => panic!("expected Bool, got {:?}", other),
        }
    }
}
