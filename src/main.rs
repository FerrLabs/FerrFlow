mod bot_token;
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
    let command_name = cli.command.name();
    let result = cli.run();

    if let Err(err) = result {
        let code = err.downcast_ref::<error_code::ErrorCode>().copied();

        let error_code_str = code.map(|c| c.to_string());
        let mut metadata = serde_json::Map::new();
        metadata.insert("command".into(), command_name.into());
        if let Some(ref code_str) = error_code_str {
            metadata.insert("error_code".into(), code_str.clone().into());
        }
        telemetry::send_event(
            telemetry::EventType::Error,
            Some(serde_json::Value::Object(metadata)),
            None,
            None,
            None,
        );

        telemetry::flush();

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

    telemetry::flush();
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
