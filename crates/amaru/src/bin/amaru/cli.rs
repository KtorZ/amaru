// Copyright 2024 PRAGMA
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

use amaru::observability::{Color, ObservabilityHints};
use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};

use crate::cmd;

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Bootstrap the node with needed data.
    ///
    /// This command simplifies the process of bootstrapping an Amaru node for any given well-known network:
    ///
    ///   - mainnet
    ///   - preprod
    ///   - preview
    ///
    /// It imports snapshots, bootstrap headers and bootstrap nonces in one step.
    #[clap(verbatim_doc_comment)]
    Bootstrap(cmd::bootstrap::Args),

    /// Dump the content of the chain database for troubleshooting purposes.
    ///
    /// This command dumps the _whole_ content of the chain database in a human-readable format:
    ///  - Headers (hash + hex-encoded body)
    ///  - Parent-child relationships between headers
    ///  - Nonces
    ///  - Blocks
    ///  - Best chain anchor, tip and length
    ///
    DumpChainDB(cmd::dump_chain_db::Args),

    /// Remove the validation status of the given blocks from the chain database.
    RemoveValidationStatus(cmd::remove_validation_status::Args),

    /// Remove the given chain fragment from the chain database.
    RemoveChain(cmd::remove_chain::Args),

    /// Dump all registered trace schemas as JSON Schema.
    ///
    /// This command outputs all registered trace schemas in JSON Schema format.
    /// Useful for documentation, tooling, and validation.
    #[command(name = "dump-traces-schema")]
    DumpTracesSchema(cmd::dump_schemas::Args),

    /// Fetch specified headers
    FetchChainHeaders(cmd::fetch_chain_headers::Args),

    /// Create the three consecutive epoch snapshots needed for bootstrap.
    CreateSnapshots(cmd::create_snapshots::Args),

    /// Migrate the chain database to the current version.
    ///
    /// This command is only relevant when one upgrades Amaru to a newer version that
    /// requires changes in the database format.
    MigrateChainDB(cmd::migrate_chain_db::Args),

    /// Reset the ledger database to the beginning of a specific epoch
    ResetToEpoch(cmd::reset_to_epoch::Args),

    /// Run the node in all its glory.
    #[command(alias = "daemon")]
    Run(cmd::run::Args),
}

impl ObservabilityHints for Command {
    fn listen_address(&self) -> Option<&str> {
        #[allow(clippy::wildcard_enum_match_arm)]
        match self {
            Command::Run(args) => Some(args.listen_address()),
            _ => None,
        }
    }
}

#[derive(Debug, Parser)]
#[clap(name = "Amaru")]
#[clap(bin_name = "amaru")]
#[clap(author, about, long_about = None)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Command,

    #[clap(long, action, env("AMARU_WITH_OPEN_TELEMETRY"))]
    pub(crate) with_open_telemetry: bool,

    #[clap(long, action, env("AMARU_WITH_JSON_TRACES"))]
    pub(crate) with_json_traces: bool,

    #[clap(long, action, env("AMARU_COLOR"))]
    pub(crate) color: Option<Color>,

    /// Do not initialize tracing library
    #[arg(short, long)]
    pub(crate) quiet: bool,
}

pub(crate) fn command(version: &'static str) -> clap::Command {
    <Cli as CommandFactory>::command().version(version)
}

pub(crate) fn parse(version: &'static str) -> Result<Cli, clap::Error> {
    let matches = command(version).get_matches();
    <Cli as FromArgMatches>::from_arg_matches(&matches)
}
