// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The native drivers' shared parts: the wall clock, CPU time from
//! `/proc`, and the host-id files through which nodes find the nodes they
//! exchange with.
//!
//! A node writes its host id, in hex, to `<dir>/<index>` once the network
//! knows its link to the relay; a node that exchanges with it waits for
//! the file. Each file is written whole and renamed into place, so a
//! reader never sees part of one.

use std::path::Path;

use strims_node_core::HostId;

/// The wall clock in Unix milliseconds.
pub fn now_ms() -> u64 {
    // The testbed measures lag against the wall clock its containers share.
    #[allow(clippy::disallowed_methods)]
    let now = std::time::SystemTime::now();
    now.duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

/// Fields 14 to 17 of `/proc/<pid>/stat` (utime, stime, cutime, cstime),
/// in clock ticks, and field 4 (the parent's pid).
fn stat(pid: &str) -> Option<([u64; 4], u32)> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    // The command name, field 2, is in parentheses and may hold spaces.
    let rest: Vec<&str> = stat
        .get(stat.rfind(')')? + 1..)?
        .split_whitespace()
        .collect();
    let field = |n: usize| rest.get(n - 3)?.parse::<u64>().ok();
    let ppid = u32::try_from(field(4)?).ok()?;
    Some(([field(14)?, field(15)?, field(16)?, field(17)?], ppid))
}

/// Linux's `USER_HZ`: clock ticks are 10 ms.
const TICK_MS: u64 = 10;

/// This process's user and system CPU time so far, in milliseconds.
/// `None` off Linux.
pub fn self_cpu_ms() -> Option<u64> {
    let ([utime, stime, _, _], _) = stat("self")?;
    Some((utime + stime) * TICK_MS)
}

/// The CPU time of `root` and every process below it, in milliseconds:
/// each one's own time and its reaped children's, so processes that ended
/// still count once their parent has reaped them.
pub fn tree_cpu_ms(root: u32) -> u64 {
    let Ok(dir) = std::fs::read_dir("/proc") else {
        return 0;
    };
    let procs: Vec<(u32, [u64; 4], u32)> = dir
        .filter_map(|e| {
            let name = e.ok()?.file_name();
            let pid: u32 = name.to_str()?.parse().ok()?;
            let (t, ppid) = stat(&pid.to_string())?;
            Some((pid, t, ppid))
        })
        .collect();
    let mut tree = vec![root];
    let mut i = 0;
    while let Some(&p) = tree.get(i) {
        tree.extend(
            procs
                .iter()
                .filter(|&&(_, _, ppid)| ppid == p)
                .map(|&(pid, _, _)| pid),
        );
        i += 1;
    }
    procs
        .iter()
        .filter(|(pid, _, _)| tree.contains(pid))
        .map(|(_, t, _)| t.iter().sum::<u64>() * TICK_MS)
        .sum()
}

/// Writes node `index`'s host id into `dir`.
pub fn write_host(dir: &Path, index: usize, id: HostId) -> std::io::Result<()> {
    let tmp = dir.join(format!(".{index}.tmp"));
    std::fs::write(&tmp, hex::encode(id.to_bytes()))?;
    std::fs::rename(tmp, dir.join(index.to_string()))
}

/// Node `index`'s host id, once its file is in `dir`.
pub fn read_host(dir: &Path, index: usize) -> Option<HostId> {
    let text = std::fs::read_to_string(dir.join(index.to_string())).ok()?;
    let b: [u8; 32] = hex::decode(text.trim()).ok()?.try_into().ok()?;
    Some(HostId::from_bytes(&b))
}
