use crate::datasets::HasSample;

pub const BYTES: &[u8] = include_bytes!("cbor/log.cbor");
pub const ID: &str = "log";

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Address {
    pub x0: u8,
    pub x1: u8,
    pub x2: u8,
    pub x3: u8,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Log {
    pub address: Address,
    pub identity: String,
    pub userid: String,
    pub date: String,
    pub request: String,
    pub code: u16,
    pub size: u64,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct BorrowLog<'a> {
    pub address: Address,
    pub identity: &'a str,
    pub userid: &'a str,
    pub date: &'a str,
    pub request: &'a str,
    pub code: u16,
    pub size: u64,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Logs {
    pub logs: Vec<Log>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct BorrowLogs<'a> {
    #[serde(borrow)]
    pub logs: Vec<BorrowLog<'a>>,
}

impl HasSample for Logs {
    #[inline(always)]
    fn sample() -> &'static [u8] {
        BYTES
    }
}

impl HasSample for BorrowLogs<'_> {
    #[inline(always)]
    fn sample() -> &'static [u8] {
        BYTES
    }
}
