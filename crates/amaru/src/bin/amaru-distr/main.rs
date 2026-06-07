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

use std::{
    fs::{self, File},
    io,
    path::{Path, PathBuf},
};

use clap::Parser;
use clap_complete::{Shell, generate};

#[allow(dead_code)]
#[path = "../amaru/cli.rs"]
mod cli;
#[allow(dead_code)]
#[path = "../amaru/cmd/mod.rs"]
mod cmd;
#[allow(dead_code)]
#[path = "../amaru/pid.rs"]
mod pid;

#[derive(Debug, Parser)]
struct Args {
    #[arg(long)]
    output_dir: PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let output_dir = args.output_dir;

    create_dir(output_dir.join("share/man/man1"))?;
    create_dir(output_dir.join("share/bash-completion/completions"))?;
    create_dir(output_dir.join("share/zsh/site-functions"))?;
    create_dir(output_dir.join("share/fish/vendor_completions.d"))?;

    render_man_page(output_dir.as_path(), env!("CARGO_PKG_VERSION"))?;
    render_completion(
        output_dir.as_path(),
        env!("CARGO_PKG_VERSION"),
        Shell::Bash,
        Path::new("share/bash-completion/completions/amaru"),
    )?;
    render_completion(
        output_dir.as_path(),
        env!("CARGO_PKG_VERSION"),
        Shell::Zsh,
        Path::new("share/zsh/site-functions/_amaru"),
    )?;
    render_completion(
        output_dir.as_path(),
        env!("CARGO_PKG_VERSION"),
        Shell::Fish,
        Path::new("share/fish/vendor_completions.d/amaru.fish"),
    )?;

    Ok(())
}

fn create_dir(path: PathBuf) -> io::Result<()> {
    fs::create_dir_all(path)
}

fn render_man_page(output_dir: &Path, version: &'static str) -> Result<(), Box<dyn std::error::Error>> {
    let command = cli::command(version);
    let man = clap_mangen::Man::new(command);
    let path = output_dir.join("share/man/man1/amaru.1");
    let mut file = File::create(path)?;
    man.render(&mut file)?;
    Ok(())
}

fn render_completion(
    output_dir: &Path,
    version: &'static str,
    shell: Shell,
    relative_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut command = cli::command(version);
    let path = output_dir.join(relative_path);
    let mut file = File::create(path)?;
    generate(shell, &mut command, "amaru", &mut file);
    Ok(())
}
