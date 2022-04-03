use serde::{Deserialize, Serialize};
use serde_with::with_prefix;

with_prefix!(order_by_prefix "order_by_");

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    Asc,
    Desc,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct QueryOrderBy<TOrderColumn> {
    pub column: TOrderColumn,
    pub direction: Order,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Query<TQueryOption, TOrderColumn>
where
    for<'a> TOrderColumn: serde::Deserialize<'a> + Serialize,
{
    pub limit: Option<i32>,
    #[serde(flatten, with = "order_by_prefix")]
    pub order_by: Option<QueryOrderBy<TOrderColumn>>,
    #[serde(flatten)]
    pub options: TQueryOption,
}
