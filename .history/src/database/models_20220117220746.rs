use serde::Serialize;

#[derive(Serialize, Que)]
pub struct Transaction {
    id: String,
    bundler: String,
    epoch: i64,
    block_promised: i64,
    block_actual: Option<i64>,
    signature: Vec<u8>,
    validated: bool
}

#[derive(Clone)]
pub struct NewTransaction<'a> {
    pub id: &'a str,
    pub bundler: &'a str,
    pub epoch: i64,
    pub block_promised: i64,
    pub block_actual: Option<i64>,
    pub signature: &'a [u8],
    pub validated: bool
}