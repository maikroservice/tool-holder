use std::collections::HashMap;

use serde_json::Value;

use crate::connector::Row;

/// Apply the YAML mapping to a single row.
///
/// `mapping` maps **ATC field name → tool field name**.
/// Only mapped fields are included in the output — unmapped tool fields are dropped.
pub fn apply_mapping(row: &Row, mapping: &HashMap<String, String>) -> HashMap<String, Value> {
    mapping
        .iter()
        .filter_map(|(atc_field, tool_field)| {
            row.get(tool_field)
                .map(|val| (atc_field.clone(), val.clone()))
        })
        .collect()
}
