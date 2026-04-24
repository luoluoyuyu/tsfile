// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Java-compatible metadata/enums/MetadataIndexNodeType module.

pub use crate::common::enums::MetadataIndexNodeType;

pub const INTERNAL_DEVICE: MetadataIndexNodeType = MetadataIndexNodeType::InternalDevice;
pub const LEAF_DEVICE: MetadataIndexNodeType = MetadataIndexNodeType::LeafDevice;
pub const INTERNAL_MEASUREMENT: MetadataIndexNodeType = MetadataIndexNodeType::InternalMeasurement;
pub const LEAF_MEASUREMENT: MetadataIndexNodeType = MetadataIndexNodeType::LeafMeasurement;
