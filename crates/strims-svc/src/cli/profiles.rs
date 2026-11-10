// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! `strims-svc profiles`: the profiles of a store no service holds, read
//! and written in place.

use std::io::Write as _;
use std::path::{Path, PathBuf};

use rand_core::SeedableRng as _;
use strims_base::rng::DetRng;
use strims_dao::auth::{list_users, remove_profile, sign_up};
use strims_dao::export::{export_profile, import_profile};
use strims_kv_redb::RedbKv;
use strims_node::{Clock as _, SystemClock};

use super::{Failure, ProfileLogin, stdout_line};
use crate::cli::connect::store_error;

/// A `profiles` command, on the store at its path.
#[derive(Debug)]
pub enum ProfilesCommand {
    /// Prints the store's user names, one a line.
    List { db: PathBuf },
    /// Signs a profile up, as the UI's sign-up does.
    Add { db: PathBuf, login: ProfileLogin },
    /// Deletes a profile and its store.
    Remove { db: PathBuf, login: ProfileLogin },
    /// Writes a profile to a new file ([`strims_dao::export`]).
    Export {
        db: PathBuf,
        login: ProfileLogin,
        out: PathBuf,
    },
    /// Adds the profile an export file holds, and prints its name.
    Import { db: PathBuf, input: PathBuf },
}

pub(crate) async fn run(command: ProfilesCommand) -> Result<(), Failure> {
    match command {
        ProfilesCommand::List { db } => {
            for name in list_users(&open(&db)?).await.map_err(failed)? {
                stdout_line(&name)?;
            }
        }
        ProfilesCommand::Add { db, login } => {
            if login.name.trim().is_empty() {
                return Err(Failure::Usage("a profile needs a name".into()));
            }
            let kv = open(&db)?;
            let mut rng = os_rng()?;
            sign_up(
                &kv,
                &login.name,
                &login.password,
                std::env::consts::OS,
                &mut rng,
                SystemClock.now(),
            )
            .await
            .map_err(|e| Failure::Failed(format!("adding {}: {e}", login.name)))?;
        }
        ProfilesCommand::Remove { db, login } => {
            remove_profile(&open(&db)?, &login.name, &login.password)
                .await
                .map_err(|e| Failure::Failed(format!("removing {}: {e}", login.name)))?;
        }
        ProfilesCommand::Export { db, login, out } => {
            let file = export_profile(&open(&db)?, &login.name, &login.password, os_rng()?)
                .await
                .map_err(|e| Failure::Failed(format!("exporting {}: {e}", login.name)))?;
            write_new(&out, &file)
                .map_err(|e| Failure::Failed(format!("writing {}: {e}", out.display())))?;
        }
        ProfilesCommand::Import { db, input } => {
            let file = std::fs::read(&input)
                .map_err(|e| Failure::Failed(format!("reading {}: {e}", input.display())))?;
            let name = import_profile(&open(&db)?, &file)
                .await
                .map_err(|e| Failure::Failed(format!("importing {}: {e}", input.display())))?;
            stdout_line(&name)?;
        }
    }
    Ok(())
}

fn failed(e: strims_dao::Error) -> Failure {
    Failure::Failed(e.to_string())
}

fn open(db: &Path) -> Result<RedbKv, Failure> {
    RedbKv::open(db).map_err(|e| Failure::Failed(store_error(db, &e.to_string())))
}

/// An rng seeded from the OS, as the service's own.
fn os_rng() -> Result<DetRng, Failure> {
    let mut seed = [0u8; 32];
    getrandom::fill(&mut seed).map_err(|e| Failure::Failed(format!("OS entropy: {e}")))?;
    Ok(DetRng::from_seed(seed))
}

/// Writes `bytes` to `path`, which mustn't exist, readable by its owner
/// alone where the OS has modes: an export is as sensitive as its store.
fn write_new(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let mut f = options.open(path)?;
    f.write_all(bytes)?;
    f.sync_all()
}
