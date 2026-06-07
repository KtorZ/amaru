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

use std::sync::LazyLock;

mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

static PACKAGE_VERSION: LazyLock<String> = LazyLock::new(|| {
    let version = format!(
        "{}.{}.{}",
        built_info::PKG_VERSION_MAJOR,
        built_info::PKG_VERSION_MINOR,
        built_info::PKG_VERSION_PATCH,
    );

    if built_info::PKG_VERSION_PRE.is_empty() {
        version
    } else {
        format!("{version}-{}", built_info::PKG_VERSION_PRE)
    }
});

static DISPLAY_VERSION: LazyLock<String> = LazyLock::new(|| match (git_commit_hash_short(), git_dirty()) {
    (Some(sha), Some(true)) => format!("{} ({sha}+dirty)", package_version()),
    (Some(sha), _) => format!("{} ({sha})", package_version()),
    _ => package_version().to_string(),
});

pub fn package_version() -> &'static str {
    PACKAGE_VERSION.as_str()
}

pub fn display_version() -> &'static str {
    DISPLAY_VERSION.as_str()
}

pub fn git_commit_hash_short() -> Option<&'static str> {
    built_info::GIT_COMMIT_HASH_SHORT
}

pub fn git_dirty() -> Option<bool> {
    built_info::GIT_DIRTY
}

pub fn target_os() -> &'static str {
    built_info::CFG_OS
}

pub fn target_arch() -> &'static str {
    built_info::CFG_TARGET_ARCH
}
