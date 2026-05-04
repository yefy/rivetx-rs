use crate::sql_value::SqlValue;
use rivetx_core_rs::rivetx_string::RivetxString;
use std::time::Duration;

pub const BATCH_SIZE: usize = 1024;
pub const TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Clone)]
pub struct QueryCond {
    pub fixed_cols: Vec<RivetxString>,
    pub fixed_vals: Vec<SqlValue>,
    pub in_cols: Vec<RivetxString>,
    pub in_vals: Vec<Vec<SqlValue>>,
    pub in_batch_size: usize,
}

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
