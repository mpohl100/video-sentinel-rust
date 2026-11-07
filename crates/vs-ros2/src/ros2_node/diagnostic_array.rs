use serde::{Deserialize, Serialize};

use super::Header;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DiagnosticArray {
    pub header: Header,
    pub status: Vec<DiagnosticStatus>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DiagnosticStatus {
    pub level: u8,
    pub name: String,
    pub message: String,
    pub hardware_id: String,
    pub values: Vec<DiagnosticKeyValue>,
}

impl DiagnosticStatus {
    pub const OK: u8 = 0;
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DiagnosticKeyValue {
    pub key: String,
    pub value: String,
}
