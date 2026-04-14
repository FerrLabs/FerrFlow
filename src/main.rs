mod changelog;
mod cli;
mod config;
mod conventional_commits;
mod error_code;
mod forge;
mod formats;
mod git;
mod hooks;
mod monorepo;
mod prerelease;
mod query;
mod status;
mod telemetry;
mod validate;
mod versioning;

use clap::Parser;
use cli::Cli;

fn main() {
    let cli = Cli::parse();
    let result = cli.run();
    telemetry::flush();

    if let Err(err) = result {
        let code = err.downcast_ref::<error_code::ErrorCode>().copied();

        if let Some(code) = code {
            let msgs: Vec<String> = err
                .chain()
                .filter(|c| c.downcast_ref::<error_code::ErrorCode>().is_none())
                .map(|c| c.to_string())
                .collect();

            eprintln!("error[{}]: {}", code, msgs[0]);
            for msg in &msgs[1..] {
                eprintln!("  {msg}");
            }
            eprintln!();
            eprintln!("  For help: {}", code.doc_url());
        } else {
            eprintln!("Error: {err:?}");
        }

        std::process::exit(1);
    }
}

#[cfg(test)]
mod test_utils {
    use std::sync::Mutex;

    /// Global lock for tests that change the process-wide working directory.
    pub static CWD_LOCK: Mutex<()> = Mutex::new(());

    pub fn with_cwd<F: FnOnce() -> anyhow::Result<()>>(
        dir: &std::path::Path,
        f: F,
    ) -> anyhow::Result<()> {
        let _lock = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let saved = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir).unwrap();
        let result = f();
        std::env::set_current_dir(&saved).unwrap();
        result
    }
}
