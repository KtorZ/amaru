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

//! Bounded reduction of the trace stream into the mobile dashboard state.

use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    time::Instant,
};

use amaru_observability::{
    RecordFields,
    amaru::{consensus, ledger, mempool, protocols},
};

use crate::trace::Record;

const PEER_LIMIT: usize = 32;
const PEER_STATE_LIMIT: usize = 128;
const PEER_IDLE_LIMIT: std::time::Duration = std::time::Duration::from_secs(600);
const RECENT_ROLLBACK_LIMIT: usize = 100;
const SEEN_SPAN_LIMIT: usize = 4_096;
const RATE_SMOOTHING: usize = 10;
const PEER_SMOOTHING: usize = 10;

/// Compact resource values sampled locally from the running Amaru process.
#[derive(Debug, Clone)]
pub struct ResourceSample {
    pub runtime_seconds: u64,
    pub cpu_percent: f64,
    pub process_memory_bytes: u64,
    pub rss_bytes: u64,
    pub virtual_bytes: u64,
    pub host_memory_used_bytes: u64,
    pub host_memory_total_bytes: u64,
    pub process_disk_read_bytes: u64,
    pub process_disk_write_bytes: u64,
    pub host_disk_read_bytes: u64,
    pub host_disk_write_bytes: u64,
}

/// Values shown by the throughput card.
#[derive(Debug, Clone, Copy, Default)]
pub struct Throughput {
    pub blocks: u64,
    pub blocks_per_second: f64,
    pub transactions: u64,
    pub transactions_per_second: f64,
}

/// Values shown by the local tip card.
#[derive(Debug, Clone)]
pub struct Tip {
    pub header_hash: String,
    pub slot: u64,
    pub block_height: u64,
    pub epoch: u64,
    pub slot_in_epoch: u64,
    pub density: f64,
    pub tx_count: u64,
}

/// Values shown by the chain-quality card.
#[derive(Debug, Clone, Copy, Default)]
pub struct ChainQuality {
    pub average_rollback_length: Option<f64>,
    pub rollback_frequency_per_second: Option<f64>,
}

/// Values shown by the mempool card.
#[derive(Debug, Clone, Copy, Default)]
pub struct Mempool {
    pub transactions: u64,
    pub size_bytes: u64,
}

/// Compact peer state shown by the mobile peer table.
#[derive(Debug, Clone)]
pub struct Peer {
    pub address: String,
    pub connected: bool,
    pub inbound: bool,
    pub outbound: bool,
    pub full_duplex: Option<bool>,
    pub full_duplex_capable: Option<bool>,
    pub rtt_micros: Option<u64>,
    pub observe_micros: Option<u64>,
    pub query_header_micros: Option<u64>,
    pub get_block_micros: Option<u64>,
    pub adopt_block_micros: Option<u64>,
    updated_at: Instant,
    observe: Mean,
    query_header: Mean,
    get_block: Mean,
    adopt_block: Mean,
}

impl Peer {
    pub fn new(address: String) -> Self {
        Self {
            address,
            connected: false,
            inbound: false,
            outbound: false,
            full_duplex: None,
            full_duplex_capable: None,
            rtt_micros: None,
            observe_micros: None,
            query_header_micros: None,
            get_block_micros: None,
            adopt_block_micros: None,
            updated_at: Instant::now(),
            observe: Mean::default(),
            query_header: Mean::default(),
            get_block: Mean::default(),
            adopt_block: Mean::default(),
        }
    }

    fn update_header_lifecycle(
        &mut self,
        observe_micros: Option<u64>,
        query_header_micros: Option<u64>,
        get_block_micros: Option<u64>,
        adopt_block_micros: Option<u64>,
    ) {
        self.observe.record(observe_micros);
        self.query_header.record(query_header_micros);
        self.get_block.record(get_block_micros);
        self.adopt_block.record(adopt_block_micros);
        self.observe_micros = self.observe.value();
        self.query_header_micros = self.query_header.value();
        self.get_block_micros = self.get_block.value();
        self.adopt_block_micros = self.adopt_block.value();
        self.updated_at = Instant::now();
    }
}

/// Stateful and bounded reduction of the trace stream.
#[derive(Debug)]
pub struct Projection {
    network: String,
    pid: u32,
    version: String,
    tip: Option<Tip>,
    system_sample: Option<ResourceSample>,
    throughput: RateCounters,
    chain_quality: ChainQuality,
    mempool: Mempool,
    peers: BTreeMap<String, Peer>,
    recent_rollbacks: VecDeque<(Instant, usize)>,
    seen_spans: BTreeSet<u64>,
    seen_span_order: VecDeque<u64>,
}

impl Projection {
    pub fn new(network: String, pid: u32, version: String) -> Self {
        Self {
            network,
            pid,
            version,
            tip: None,
            system_sample: None,
            throughput: RateCounters::default(),
            chain_quality: ChainQuality::default(),
            mempool: Mempool::default(),
            peers: BTreeMap::new(),
            recent_rollbacks: VecDeque::new(),
            seen_spans: BTreeSet::new(),
            seen_span_order: VecDeque::new(),
        }
    }

    pub fn network(&self) -> &str {
        &self.network
    }

    pub fn pid(&self) -> u32 {
        self.pid
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn set_system_sample(&mut self, sample: Option<ResourceSample>) {
        self.system_sample = sample;
        self.throughput.sample();
        self.refresh_chain_quality();
        self.trim_peers();
    }

    pub fn system_sample(&self) -> Option<&ResourceSample> {
        self.system_sample.as_ref()
    }

    pub fn throughput(&self) -> Throughput {
        self.throughput.values()
    }

    pub fn tip(&self) -> Option<&Tip> {
        self.tip.as_ref()
    }

    pub fn chain_quality(&self) -> &ChainQuality {
        &self.chain_quality
    }

    pub fn mempool(&self) -> &Mempool {
        &self.mempool
    }

    /// Peers are ordered by ascending RTT and truncated before wire encoding.
    pub fn peers(&self) -> Vec<Peer> {
        let mut peers = self.peers.values().cloned().collect::<Vec<_>>();
        peers.sort_by_key(|peer| peer.rtt_micros.unwrap_or(u64::MAX));
        peers.truncate(PEER_LIMIT);
        peers
    }

    #[cfg(test)]
    pub fn insert_peer(&mut self, peer: Peer) {
        self.peers.insert(peer.address.clone(), peer);
    }

    /// Applies one JSON trace record. Historical replay restores current state but does not
    /// inflate the counters shown as work performed since the bridge started.
    pub fn apply(&mut self, record: Record, historical: bool) {
        if !self.accept_span(&record) {
            return;
        }
        let Some(name) = record.name() else {
            return;
        };

        if ledger::tip::UPDATE::matches(record.target(), name) {
            self.tip = tip(&record);
        } else if ledger::state::ROLL_FORWARD::matches(record.target(), name) {
            if !historical {
                self.throughput.blocks.record(1);
            }
        } else if ledger::transaction::VALIDATE::matches(record.target(), name) {
            if !historical
                && record.has_parent(ledger::state::ROLL_FORWARD::NAME)
                && record.has_parent(ledger::rules::BLOCK::NAME)
                && record.has_parent(ledger::rules::PHASE_ONE::NAME)
            {
                self.throughput.transactions.record(1);
            }
        } else if ledger::state::SWITCH_TO_FORK::matches(record.target(), name) {
            if !historical && let Some(length) = record.usize(ledger::state::SWITCH_TO_FORK::FIELD_ROLLBACK_LENGTH) {
                self.recent_rollbacks.push_back((Instant::now(), length));
                while self.recent_rollbacks.len() > RECENT_ROLLBACK_LIMIT {
                    self.recent_rollbacks.pop_front();
                }
            }
        } else if mempool::state::UPDATE::matches(record.target(), name) {
            self.mempool = Mempool {
                transactions: record.u64(mempool::state::UPDATE::FIELD_TX_COUNT).unwrap_or_default(),
                size_bytes: record.u64(mempool::state::UPDATE::FIELD_SIZE_BYTES).unwrap_or_default(),
            };
        } else if protocols::peer_selection::peer::CONNECTED::matches(record.target(), name) {
            self.peer_connected(&record);
        } else if protocols::peer_selection::peer::DISCONNECTED::matches(record.target(), name) {
            self.peer_disconnected(&record);
        } else if protocols::keepalive::peer::ROUND_TRIP::matches(record.target(), name) {
            self.peer_round_trip(&record);
        } else if consensus::perf::header::LIFECYCLE::matches(record.target(), name) {
            self.peer_header_lifecycle(&record);
        }
    }

    fn accept_span(&mut self, record: &Record) -> bool {
        let Some(id) = record.id() else {
            return true;
        };
        if !self.seen_spans.insert(id) {
            return false;
        }
        self.seen_span_order.push_back(id);
        if self.seen_span_order.len() > SEEN_SPAN_LIMIT
            && let Some(oldest) = self.seen_span_order.pop_front()
        {
            self.seen_spans.remove(&oldest);
        }
        true
    }

    fn peer_connected(&mut self, record: &Record) {
        let Some(address) = record.str(protocols::peer_selection::peer::CONNECTED::FIELD_PEER) else {
            return;
        };
        let peer = self.peer_mut(address);
        peer.connected = true;
        match record.str(protocols::peer_selection::peer::CONNECTED::FIELD_DIRECTION) {
            Some("Inbound") => peer.inbound = true,
            Some("Outbound") => peer.outbound = true,
            _ => {}
        }
        peer.full_duplex = record.bool(protocols::peer_selection::peer::CONNECTED::FIELD_FULL_DUPLEX);
        peer.full_duplex_capable = record.bool(protocols::peer_selection::peer::CONNECTED::FIELD_FULL_DUPLEX_CAPABLE);
        peer.updated_at = Instant::now();
    }

    fn peer_disconnected(&mut self, record: &Record) {
        let Some(address) = record.str(protocols::peer_selection::peer::DISCONNECTED::FIELD_PEER) else {
            return;
        };
        if let Some(peer) = self.peers.get_mut(address) {
            peer.connected = false;
            peer.updated_at = Instant::now();
        }
    }

    fn peer_round_trip(&mut self, record: &Record) {
        let Some(address) = record.str(protocols::keepalive::peer::ROUND_TRIP::FIELD_PEER) else {
            return;
        };
        let Some(rtt_micros) = record.u64(protocols::keepalive::peer::ROUND_TRIP::FIELD_ROUND_TRIP_MICROS) else {
            return;
        };
        let peer = self.peer_mut(address);
        peer.connected = true;
        peer.outbound = true;
        peer.rtt_micros = Some(rtt_micros);
        peer.updated_at = Instant::now();
    }

    fn peer_header_lifecycle(&mut self, record: &Record) {
        if record.str(consensus::perf::header::LIFECYCLE::FIELD_OUTCOME) != Some("valid") {
            return;
        }
        let Some(address) = record.str(consensus::perf::header::LIFECYCLE::FIELD_PEER) else {
            return;
        };
        let query_header = record.u64(consensus::perf::header::LIFECYCLE::FIELD_BLOCK_FETCH_WAIT_MICROS);
        let get_block = record.u64(consensus::perf::header::LIFECYCLE::FIELD_BLOCK_FETCH_MICROS);
        let adopt_block = record
            .u64(consensus::perf::header::LIFECYCLE::FIELD_FORWARD_MICROS)
            .zip(query_header)
            .zip(get_block)
            .map(|((forward, query), get)| forward.saturating_sub(query.saturating_add(get)));
        self.peer_mut(address).update_header_lifecycle(
            record.u64(consensus::perf::header::LIFECYCLE::FIELD_SLOT_START_TO_HEADER_MICROS),
            query_header,
            get_block,
            adopt_block,
        );
    }

    fn peer_mut(&mut self, address: &str) -> &mut Peer {
        if !self.peers.contains_key(address)
            && self.peers.len() >= PEER_STATE_LIMIT
            && let Some(oldest) =
                self.peers.values().min_by_key(|peer| peer.updated_at).map(|peer| peer.address.clone())
        {
            self.peers.remove(&oldest);
        }
        self.peers.entry(address.to_owned()).or_insert_with(|| Peer::new(address.to_owned()))
    }

    fn trim_peers(&mut self) {
        let now = Instant::now();
        self.peers
            .retain(|_, peer| peer.connected || now.saturating_duration_since(peer.updated_at) <= PEER_IDLE_LIMIT);
    }

    fn refresh_chain_quality(&mut self) {
        let now = Instant::now();
        self.recent_rollbacks
            .retain(|(at, _)| now.saturating_duration_since(*at) <= std::time::Duration::from_secs(600));
        if self.recent_rollbacks.is_empty() {
            self.chain_quality = ChainQuality::default();
            return;
        }
        let total = self.recent_rollbacks.iter().map(|(_, length)| *length as f64).sum::<f64>();
        self.chain_quality.average_rollback_length = Some(total / self.recent_rollbacks.len() as f64);
        self.chain_quality.rollback_frequency_per_second = Some(self.recent_rollbacks.len() as f64 / 600.0);
    }
}

fn tip(record: &Record) -> Option<Tip> {
    Some(Tip {
        header_hash: record.str(ledger::tip::UPDATE::FIELD_HEADER_HASH)?.to_owned(),
        slot: record.u64(ledger::tip::UPDATE::FIELD_SLOT)?,
        block_height: record.u64(ledger::tip::UPDATE::FIELD_BLOCK_HEIGHT)?,
        epoch: record.u64(ledger::tip::UPDATE::FIELD_EPOCH)?,
        slot_in_epoch: record.u64(ledger::tip::UPDATE::FIELD_SLOT_IN_EPOCH)?,
        density: record.f64(ledger::tip::UPDATE::FIELD_DENSITY)?,
        tx_count: record.u64(ledger::tip::UPDATE::FIELD_TX_COUNT)?,
    })
}

#[derive(Debug, Default)]
struct RateCounters {
    blocks: Rate,
    transactions: Rate,
}

impl RateCounters {
    fn sample(&mut self) {
        self.blocks.sample();
        self.transactions.sample();
    }

    fn values(&self) -> Throughput {
        Throughput {
            blocks: self.blocks.total,
            blocks_per_second: self.blocks.per_second.value.unwrap_or_default(),
            transactions: self.transactions.total,
            transactions_per_second: self.transactions.per_second.value.unwrap_or_default(),
        }
    }
}

#[derive(Debug, Default)]
struct Rate {
    total: u64,
    pending: u64,
    sampled_at: Option<Instant>,
    per_second: Mean,
}

impl Rate {
    fn record(&mut self, count: u64) {
        self.total = self.total.saturating_add(count);
        self.pending = self.pending.saturating_add(count);
    }

    fn sample(&mut self) {
        let now = Instant::now();
        let Some(sampled_at) = self.sampled_at.replace(now) else {
            self.pending = 0;
            return;
        };
        let elapsed = now.saturating_duration_since(sampled_at).as_secs_f64();
        if elapsed > 0.0 {
            self.per_second.record_value(self.pending as f64 / elapsed, RATE_SMOOTHING);
        }
        self.pending = 0;
    }
}

#[derive(Debug, Clone, Default)]
struct Mean {
    value: Option<f64>,
}

impl Mean {
    fn record(&mut self, value: Option<u64>) {
        if let Some(value) = value {
            self.record_value(value as f64, PEER_SMOOTHING);
        }
    }

    fn record_value(&mut self, sample: f64, smoothing: usize) {
        let alpha = 2.0 / (smoothing.max(1) as f64 + 1.0);
        self.value = Some(match self.value {
            Some(value) => alpha * sample + (1.0 - alpha) * value,
            None => sample,
        });
    }

    fn value(&self) -> Option<u64> {
        self.value.map(|value| value.round() as u64)
    }
}

#[cfg(test)]
mod tests {
    use amaru_observability::amaru::ledger;

    use super::*;

    fn record(fields: &str) -> Record {
        Record::parse(&format!(
            r#"{{"target":"{}","fields":{{"message":"{}",{fields}}}}}"#,
            ledger::tip::UPDATE::TARGET,
            ledger::tip::UPDATE::NAME
        ))
        .expect("record")
    }

    #[test]
    fn reconstructs_the_tip_from_typed_schema_fields() {
        let mut projection = Projection::new("preview".into(), 1, "amaru 0.0.0".into());
        projection.apply(
            record(r#""slot":100,"header_hash":"abc","block_height":4,"tx_count":2,"epoch":1,"slot_in_epoch":10,"density":0.05,"current_kes_period":1,"remaining_kes_periods":2"#),
            false,
        );

        let tip = projection.tip().expect("tip");
        assert_eq!(tip.slot, 100);
        assert_eq!(tip.header_hash, "abc");
    }
}
