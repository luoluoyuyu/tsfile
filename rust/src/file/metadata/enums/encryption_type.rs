// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Java-compatible metadata/enums/EncryptionType module.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EncryptionType {
    Unencrypted,
}

impl EncryptionType {
    pub fn serialize(self) -> u8 {
        match self {
            EncryptionType::Unencrypted => 0,
        }
    }

    pub fn deserialize(value: u8) -> Option<Self> {
        match value {
            0 => Some(EncryptionType::Unencrypted),
            _ => None,
        }
    }

    pub fn class_name(self) -> &'static str {
        match self {
            EncryptionType::Unencrypted => "org.apache.tsfile.encrypt.UNENCRYPTED",
        }
    }
}
