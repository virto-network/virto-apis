use serde::{Deserialize, Serialize};
use serde_with::with_prefix;

#[derive(Serialize, Deserialize, Debug)]
pub struct Query<Opts, OrdF>
where
    for<'a> OrdF: serde::Deserialize<'a> + Serialize,
{
    pub limit: Option<u16>,
    #[serde(flatten, with = "order_by_prefix")]
    pub order_by: Option<OrderBy<OrdF>>,
    #[serde(flatten)]
    pub options: Opts,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    Asc,
    Desc,
}

with_prefix!(order_by_prefix "order_by_");

#[derive(Serialize, Deserialize, Debug)]
pub struct OrderBy<F> {
    pub field: F,
    pub direction: Order,
}

impl<Opts, OrdF> From<()> for Query<Opts, OrdF>
where
    Opts: Default,
    for<'a> OrdF: serde::Deserialize<'a> + Serialize,
{
    fn from(_: ()) -> Self {
        Query {
            limit: None,
            order_by: None,
            options: Default::default(),
        }
    }
}
