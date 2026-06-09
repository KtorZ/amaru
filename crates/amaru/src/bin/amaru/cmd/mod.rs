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

use std::{ops::Deref, str::FromStr};

use amaru_kernel::{HeaderHash, Point};

pub(crate) mod bootstrap;
pub(crate) mod create_snapshots;
pub(crate) mod distr;
pub(crate) mod dump_chain_db;
pub(crate) mod dump_schemas;
pub(crate) mod fetch_chain_headers;
pub(crate) mod migrate_chain_db;
pub(crate) mod remove_chain;
pub(crate) mod remove_validation_status;
pub(crate) mod reset_to_epoch;
pub(crate) mod run;

#[derive(Debug, Clone)]
struct PointOrHash(HeaderHash);
impl FromStr for PointOrHash {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<Point>().map(|p| p.hash()).or_else(|_| s.parse::<HeaderHash>().map_err(|e| e.to_string())).map(Self)
    }
}
impl Deref for PointOrHash {
    type Target = HeaderHash;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
