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

//! Versioned wire format for the mobile telemetry characteristic.

use std::time::{SystemTime, UNIX_EPOCH};

use minicbor::{Encode, Encoder};

use crate::projection::{ChainQuality, Mempool, Peer, Projection, ResourceSample, Throughput, Tip};

/// Current CBOR snapshot version.
pub const SNAPSHOT_VERSION: u8 = 1;

/// One-byte marker used to reject unrelated GATT notifications quickly.
pub const FRAGMENT_MAGIC: u8 = 0xa7;

/// Header length for each BLE notification fragment.
pub const FRAGMENT_HEADER_LEN: usize = 8;

/// Payload small enough for conservative iOS BLE MTUs after the fragment header.
pub const FRAGMENT_PAYLOAD_LEN: usize = 160;

/// The bridge never emits more than this payload per second.
pub const MAX_SNAPSHOT_BYTES: usize = 7 * 1_024;

/// Encodes the latest complete state as a CBOR array.
///
/// A CBOR array keeps the client decoder compact. Field order is part of the versioned
/// contract; a new order requires a new [`SNAPSHOT_VERSION`].
pub fn snapshot_bytes(projection: &Projection, sequence: u32) -> Vec<u8> {
    let mut peers = projection.peers();
    loop {
        let snapshot = Snapshot::from_projection(projection, sequence, &peers);
        let mut bytes = Vec::with_capacity(2_048);
        Encoder::new(&mut bytes).encode(snapshot).expect("encoding mobile telemetry cannot fail");
        if bytes.len() <= MAX_SNAPSHOT_BYTES || peers.is_empty() {
            return bytes;
        }
        peers.pop();
    }
}

/// Splits one CBOR snapshot into independently transportable GATT notifications.
pub fn fragment(sequence: u32, snapshot: &[u8]) -> Vec<Vec<u8>> {
    let chunks = snapshot.chunks(FRAGMENT_PAYLOAD_LEN).collect::<Vec<_>>();
    let chunk_count = u8::try_from(chunks.len()).expect("snapshot fragment count must fit in u8");

    chunks
        .into_iter()
        .enumerate()
        .map(|(index, chunk)| {
            let mut frame = Vec::with_capacity(FRAGMENT_HEADER_LEN + chunk.len());
            frame.push(FRAGMENT_MAGIC);
            frame.push(SNAPSHOT_VERSION);
            frame.extend_from_slice(&sequence.to_be_bytes());
            frame.push(u8::try_from(index).expect("snapshot fragment index must fit in u8"));
            frame.push(chunk_count);
            frame.extend_from_slice(chunk);
            frame
        })
        .collect()
}

#[derive(Encode)]
#[cbor(array)]
struct Snapshot<'a> {
    #[n(0)]
    version: u8,
    #[n(1)]
    sequence: u32,
    #[n(2)]
    generated_at_millis: u64,
    #[n(3)]
    node: Node<'a>,
    #[n(4)]
    resource: Option<Resource>,
    #[n(5)]
    throughput: ThroughputWire,
    #[n(6)]
    tip: Option<TipWire<'a>>,
    #[n(7)]
    chain_quality: ChainQualityWire,
    #[n(8)]
    mempool: MempoolWire,
    #[n(9)]
    peers: Vec<PeerWire<'a>>,
}

impl<'a> Snapshot<'a> {
    fn from_projection(projection: &'a Projection, sequence: u32, peers: &'a [Peer]) -> Self {
        Self {
            version: SNAPSHOT_VERSION,
            sequence,
            generated_at_millis: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
                .try_into()
                .unwrap_or(u64::MAX),
            node: Node::from_projection(projection),
            resource: projection.system_sample().map(Resource::from_sample),
            throughput: ThroughputWire::from_throughput(&projection.throughput()),
            tip: projection.tip().map(TipWire::from_tip),
            chain_quality: ChainQualityWire::from_chain_quality(projection.chain_quality()),
            mempool: MempoolWire::from_mempool(projection.mempool()),
            peers: peers.iter().map(PeerWire::from_peer).collect(),
        }
    }
}

#[derive(Encode)]
#[cbor(array)]
struct Node<'a> {
    #[n(0)]
    network: &'a str,
    #[n(1)]
    pid: u32,
    #[n(2)]
    version: &'a str,
    #[n(3)]
    uptime_seconds: Option<u64>,
}

impl<'a> Node<'a> {
    fn from_projection(projection: &'a Projection) -> Self {
        Self {
            network: projection.network(),
            pid: projection.pid(),
            version: projection.version(),
            uptime_seconds: projection.system_sample().map(|sample| sample.runtime_seconds),
        }
    }
}

#[derive(Encode)]
#[cbor(array)]
struct Resource {
    #[n(0)]
    cpu_percent: f64,
    #[n(1)]
    process_memory_bytes: u64,
    #[n(2)]
    rss_bytes: u64,
    #[n(3)]
    virtual_bytes: u64,
    #[n(4)]
    host_memory_used_bytes: u64,
    #[n(5)]
    host_memory_total_bytes: u64,
    #[n(6)]
    process_disk_read_bytes: u64,
    #[n(7)]
    process_disk_write_bytes: u64,
    #[n(8)]
    host_disk_read_bytes: u64,
    #[n(9)]
    host_disk_write_bytes: u64,
}

impl Resource {
    fn from_sample(sample: &ResourceSample) -> Self {
        Self {
            cpu_percent: sample.cpu_percent,
            process_memory_bytes: sample.process_memory_bytes,
            rss_bytes: sample.rss_bytes,
            virtual_bytes: sample.virtual_bytes,
            host_memory_used_bytes: sample.host_memory_used_bytes,
            host_memory_total_bytes: sample.host_memory_total_bytes,
            process_disk_read_bytes: sample.process_disk_read_bytes,
            process_disk_write_bytes: sample.process_disk_write_bytes,
            host_disk_read_bytes: sample.host_disk_read_bytes,
            host_disk_write_bytes: sample.host_disk_write_bytes,
        }
    }
}

#[derive(Encode)]
#[cbor(array)]
struct ThroughputWire {
    #[n(0)]
    blocks: u64,
    #[n(1)]
    blocks_per_second: f64,
    #[n(2)]
    transactions: u64,
    #[n(3)]
    transactions_per_second: f64,
}

impl ThroughputWire {
    fn from_throughput(throughput: &Throughput) -> Self {
        Self {
            blocks: throughput.blocks,
            blocks_per_second: throughput.blocks_per_second,
            transactions: throughput.transactions,
            transactions_per_second: throughput.transactions_per_second,
        }
    }
}

#[derive(Encode)]
#[cbor(array)]
struct TipWire<'a> {
    #[n(0)]
    header_hash: &'a str,
    #[n(1)]
    slot: u64,
    #[n(2)]
    block_height: u64,
    #[n(3)]
    epoch: u64,
    #[n(4)]
    slot_in_epoch: u64,
    #[n(5)]
    density: f64,
    #[n(6)]
    tx_count: u64,
}

impl<'a> TipWire<'a> {
    fn from_tip(tip: &'a Tip) -> Self {
        Self {
            header_hash: &tip.header_hash,
            slot: tip.slot,
            block_height: tip.block_height,
            epoch: tip.epoch,
            slot_in_epoch: tip.slot_in_epoch,
            density: tip.density,
            tx_count: tip.tx_count,
        }
    }
}

#[derive(Encode)]
#[cbor(array)]
struct ChainQualityWire {
    #[n(0)]
    average_rollback_length: Option<f64>,
    #[n(1)]
    rollback_frequency_per_second: Option<f64>,
}

impl ChainQualityWire {
    fn from_chain_quality(chain_quality: &ChainQuality) -> Self {
        Self {
            average_rollback_length: chain_quality.average_rollback_length,
            rollback_frequency_per_second: chain_quality.rollback_frequency_per_second,
        }
    }
}

#[derive(Encode)]
#[cbor(array)]
struct MempoolWire {
    #[n(0)]
    transactions: u64,
    #[n(1)]
    size_bytes: u64,
}

impl MempoolWire {
    fn from_mempool(mempool: &Mempool) -> Self {
        Self { transactions: mempool.transactions, size_bytes: mempool.size_bytes }
    }
}

#[derive(Encode)]
#[cbor(array)]
struct PeerWire<'a> {
    #[n(0)]
    address: &'a str,
    #[n(1)]
    connected: bool,
    #[n(2)]
    inbound: bool,
    #[n(3)]
    outbound: bool,
    #[n(4)]
    full_duplex: Option<bool>,
    #[n(5)]
    full_duplex_capable: Option<bool>,
    #[n(6)]
    rtt_micros: Option<u64>,
    #[n(7)]
    observe_micros: Option<u64>,
    #[n(8)]
    query_header_micros: Option<u64>,
    #[n(9)]
    get_block_micros: Option<u64>,
    #[n(10)]
    adopt_block_micros: Option<u64>,
}

impl<'a> PeerWire<'a> {
    fn from_peer(peer: &'a Peer) -> Self {
        Self {
            address: &peer.address,
            connected: peer.connected,
            inbound: peer.inbound,
            outbound: peer.outbound,
            full_duplex: peer.full_duplex,
            full_duplex_capable: peer.full_duplex_capable,
            rtt_micros: peer.rtt_micros,
            observe_micros: peer.observe_micros,
            query_header_micros: peer.query_header_micros,
            get_block_micros: peer.get_block_micros,
            adopt_block_micros: peer.adopt_block_micros,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projection::{Peer, Projection};

    #[test]
    fn wire_output_stays_under_the_per_second_budget() {
        let mut projection = Projection::new("mainnet".into(), 42, "amaru 0.0.0".into());
        for index in 0..512 {
            projection.insert_peer(Peer::new(format!("[2001:db8::{index}]:3001")));
        }

        let bytes = snapshot_bytes(&projection, 7);
        assert!(bytes.len() <= MAX_SNAPSHOT_BYTES);

        let fragments = fragment(7, &bytes);
        assert!(fragments.iter().all(|frame| frame.len() <= FRAGMENT_HEADER_LEN + FRAGMENT_PAYLOAD_LEN));
        assert!(fragments.iter().map(Vec::len).sum::<usize>() <= 10 * 1_024);
    }
}
