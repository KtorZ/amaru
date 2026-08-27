// Copyright 2026 PRAGMA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Types for Amaru's newline-delimited JSON trace format.

use std::collections::BTreeMap;

use amaru_observability::{
    RecordFields,
    amaru::{consensus, ledger, mempool, protocols},
};
use serde::Deserialize;
use serde_json::Value;

/// Fast prefilter for the small set of schemas that contributes to the mobile dashboard.
///
/// The generated names retain compile-time coupling to Amaru's telemetry contract while
/// avoiding JSON deserialization for unrelated high-volume trace records.
pub fn is_relevant(line: &str) -> bool {
    [
        ledger::tip::UPDATE::NAME,
        ledger::state::ROLL_FORWARD::NAME,
        ledger::state::SWITCH_TO_FORK::NAME,
        ledger::transaction::VALIDATE::NAME,
        mempool::state::UPDATE::NAME,
        protocols::peer_selection::peer::CONNECTED::NAME,
        protocols::peer_selection::peer::DISCONNECTED::NAME,
        protocols::keepalive::peer::ROUND_TRIP::NAME,
        consensus::perf::header::LIFECYCLE::NAME,
    ]
    .into_iter()
    .any(|name| line.contains(name))
}

/// Minimal view of a JSON trace record, implementing the generated schema accessors.
#[derive(Debug, Deserialize)]
pub struct Record {
    target: String,
    fields: BTreeMap<String, Value>,
    #[serde(default)]
    parents: Vec<String>,
    #[serde(default)]
    id: Option<u64>,
}

impl Record {
    pub fn parse(line: &str) -> Option<Self> {
        serde_json::from_str(line).ok()
    }

    pub fn target(&self) -> &str {
        &self.target
    }

    /// The schema macros put their generated name in this reserved JSON field.
    pub fn name(&self) -> Option<&str> {
        self.str("message")
    }

    /// Returns whether this is the first JSON lifecycle record for a span.
    ///
    /// The JSON formatter emits both `ENTER` and `EXIT` records. They have the same
    /// span id and schema name, so reducers must process only the first one.
    pub fn id(&self) -> Option<u64> {
        self.id
    }

    /// Returns whether the record was emitted below the named parent span.
    pub fn has_parent(&self, name: &str) -> bool {
        self.parents.iter().any(|parent| parent == name)
    }
}

impl RecordFields for Record {
    fn bool(&self, name: &str) -> Option<bool> {
        self.fields.get(name)?.as_bool()
    }

    fn f64(&self, name: &str) -> Option<f64> {
        self.fields.get(name)?.as_f64()
    }

    fn i64(&self, name: &str) -> Option<i64> {
        self.fields
            .get(name)?
            .as_i64()
            .or_else(|| self.fields.get(name)?.as_u64().and_then(|value| value.try_into().ok()))
    }

    fn str(&self, name: &str) -> Option<&str> {
        self.fields.get(name)?.as_str()
    }

    fn u64(&self, name: &str) -> Option<u64> {
        self.fields
            .get(name)?
            .as_u64()
            .or_else(|| self.fields.get(name)?.as_i64().and_then(|value| value.try_into().ok()))
    }
}

#[cfg(test)]
mod tests {
    use amaru_observability::amaru::ledger;

    use super::*;

    #[test]
    fn parses_schema_name_and_typed_fields() {
        let record =
            Record::parse(r#"{"target":"amaru::ledger","fields":{"message":"tip.update","slot":42,"density":0.05}}"#)
                .expect("record");

        assert!(ledger::tip::UPDATE::matches(record.target(), record.name().expect("name")));
        assert_eq!(record.u64(ledger::tip::UPDATE::FIELD_SLOT), Some(42));
        assert_eq!(record.f64(ledger::tip::UPDATE::FIELD_DENSITY), Some(0.05));
    }
}
