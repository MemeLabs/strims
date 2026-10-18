// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! `strims-svc`: runs a profile's node and serves the UI's frontend RPC
//! over WebSocket until interrupted. See [`args::USAGE`].

mod args;

use std::process::ExitCode;

use strims_svc::Svc;
use tokio::task::LocalSet;
use tracing_subscriber::EnvFilter;

fn main() -> ExitCode {
    let config = match args::parse(std::env::args().skip(1)) {
        Ok(args::Command::Run(config)) => *config,
        Ok(args::Command::Help) => {
            println!("{}", args::USAGE);
            return ExitCode::SUCCESS;
        }
        Err(e) => {
            eprintln!("strims-svc: {e}\n\n{}", args::USAGE);
            return ExitCode::from(2);
        }
    };
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();
    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("strims-svc: starting the runtime: {e}");
            return ExitCode::FAILURE;
        }
    };
    rt.block_on(LocalSet::new().run_until(async {
        let path = config.api_path.clone();
        let svc = match Svc::start(config).await {
            Ok(svc) => svc,
            Err(e) => {
                eprintln!("strims-svc: {e}");
                return ExitCode::FAILURE;
            }
        };
        tracing::info!(address = %svc.api_addr(), %path, "serving the UI");
        if let Err(e) = tokio::signal::ctrl_c().await {
            tracing::warn!(error = %e, "can't wait for ctrl-c; running until killed");
            std::future::pending::<()>().await;
        }
        tracing::info!("stopping");
        svc.stop().await;
        ExitCode::SUCCESS
    }))
}
