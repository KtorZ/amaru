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

//! Narrow local handoff for the Bluetooth power-off command.

use anyhow::Context;
use tokio::process::Command;

const SUDO: &str = "/usr/bin/sudo";
const SYSTEMCTL: &str = "/usr/bin/systemctl";
const SERVICE: &str = "amaru-mobile-poweroff.service";

/// Exact, versioned control value accepted by the power-off characteristic.
///
/// This is not a secret or an authentication mechanism. It merely prevents a
/// malformed or unrelated GATT write from becoming a shutdown request.
pub const COMMAND: &[u8] = b"amaru/power-off/v1";

/// True only for the one control value supported by the bridge.
pub fn is_command(value: &[u8]) -> bool {
    value == COMMAND
}

/// Asks the dedicated root-owned systemd unit to power off the host.
///
/// The exact command is constrained by the installed sudoers rule. `--no-block`
/// acknowledges the accepted request before shutdown tears down D-Bus.
pub async fn schedule() -> anyhow::Result<()> {
    let status = Command::new(SUDO)
        .args(["--non-interactive", SYSTEMCTL, "--no-block", "start", SERVICE])
        .status()
        .await
        .context("start power-off service")?;

    anyhow::ensure!(status.success(), "power-off service request failed with {status}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_the_fixed_command() {
        assert!(is_command(COMMAND));
        assert!(!is_command(b"amaru/power-off/v2"));
        assert!(!is_command(b"poweroff"));
    }
}
