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

use amaru::{
    observability::{Color, setup_observability},
    panic::panic_handler,
    version,
};
use tracing::info;

mod cli;
mod cmd;
mod pid;

// TODO(rkuhn): properly measure and design the Tokio runtime setup we need.
// (probably one runtime for network with 1-2 threads, one for CPU-bound tasks according to parallelism,
// one for running the consensus pipeline incl. Store access with 2+ threads)
#[expect(clippy::unwrap_used)]
#[tokio::main(worker_threads = 4)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    panic_handler();

    let args = cli::parse(version::display_version())?;

    let skip_logging =
        args.quiet || matches!(args.command, cli::Command::DumpTracesSchema(_) | cli::Command::ShellCompletions(_));

    let (metrics, teardown) = if skip_logging {
        (None, Box::new(|| Ok(())) as Box<dyn FnOnce() -> Result<(), Box<dyn std::error::Error>>>)
    } else {
        let (m, t) = setup_observability(
            args.with_open_telemetry,
            args.with_json_traces,
            Color::is_enabled(args.color),
            &args.command,
        );
        (Some(m), t)
    };

    info!(
        with_open_telemetry = args.with_open_telemetry,
        with_json_traces = args.with_json_traces,
        "Started with global arguments"
    );

    let result = match args.command {
        cli::Command::Run(args) => cmd::run::run(args, metrics.unwrap()).await,
        cli::Command::Bootstrap(args) => cmd::bootstrap::run(args).await,
        cli::Command::FetchChainHeaders(args) => cmd::fetch_chain_headers::run(args).await,
        cli::Command::CreateSnapshots(args) => cmd::create_snapshots::run(args).await,
        cli::Command::ShellCompletions(args) => cmd::distr::run(args).await,
        cli::Command::DumpChainDB(args) => cmd::dump_chain_db::run(args).await,
        cli::Command::RemoveValidationStatus(args) => cmd::remove_validation_status::run(args).await,
        cli::Command::RemoveChain(args) => cmd::remove_chain::run(args).await,
        cli::Command::DumpTracesSchema(args) => cmd::dump_schemas::run(args).await,
        cli::Command::MigrateChainDB(args) => cmd::migrate_chain_db::run(args).await,
        cli::Command::ResetToEpoch(args) => cmd::reset_to_epoch::run(args).await,
    };

    // TODO: we might also want to integrate this into a graceful shutdown system, and into a panic hook
    if let Err(report) = teardown() {
        eprintln!("Failed to teardown tracing: {report}");
    }

    result
}
