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

#![cfg(unix)]

use std::{error::Error, process::Output, time::Duration};

use assert_cmd::{Command, cargo::cargo_bin};
use tempfile::TempDir;

fn run_under_low_fd_limit(color: &str) -> Result<Output, Box<dyn Error>> {
    let root = TempDir::new()?;
    let ledger_dir = root.path().join("ledger.preprod.db");
    let chain_dir = root.path().join("chain.preprod.db");
    let amaru = cargo_bin("amaru");

    let mut command = Command::new("sh");
    command
        .arg("-c")
        .arg("ulimit -n 256; exec \"$@\"")
        .arg("sh")
        .arg(amaru)
        .arg("--color")
        .arg(color)
        .arg("run")
        .arg("--peer-address")
        .arg("127.0.0.1:65532")
        .arg("--ledger-dir")
        .arg(&ledger_dir)
        .arg("--chain-dir")
        .arg(&chain_dir)
        .env("AMARU_NETWORK", "preprod")
        .timeout(Duration::from_secs(15));

    Ok(command.output()?)
}

fn combined_output(output: &Output) -> Vec<u8> {
    let mut bytes = output.stdout.clone();
    bytes.extend_from_slice(&output.stderr);
    bytes
}

fn contains_ansi_escape(bytes: &[u8]) -> bool {
    bytes.windows(2).any(|window| window == b"\x1b[")
}

#[test]
fn explains_fd_limit_is_too_low() -> Result<(), Box<dyn Error>> {
    let output = run_under_low_fd_limit("never")?;
    let rendered = combined_output(&output);
    let rendered = String::from_utf8_lossy(&rendered);

    assert!(!output.status.success());
    assert!(rendered.contains("Increase the limit for open files before starting Amaru"));

    Ok(())
}

#[test]
fn no_color_when_color_is_never() -> Result<(), Box<dyn Error>> {
    let output = run_under_low_fd_limit("never")?;
    let rendered = combined_output(&output);

    assert!(!output.status.success());
    assert!(
        !contains_ansi_escape(&rendered),
        "found ANSI escape codes in output:\n{}",
        String::from_utf8_lossy(&rendered)
    );

    Ok(())
}

#[test]
fn color_when_color_is_always() -> Result<(), Box<dyn Error>> {
    let output = run_under_low_fd_limit("always")?;
    let rendered = combined_output(&output);

    assert!(!output.status.success());
    assert!(
        contains_ansi_escape(&rendered),
        "expected ANSI escape codes in output but found none:\n{}",
        String::from_utf8_lossy(&rendered)
    );

    Ok(())
}
