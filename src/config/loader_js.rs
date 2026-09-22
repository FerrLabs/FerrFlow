use anyhow::{Context, Result};
use std::path::Path;

use crate::error_code::{self, ErrorCodeExt};

use super::Config;

pub(crate) const JS_CONFIG_FILENAME: &str = "ferrflow.js";
pub(crate) const TS_CONFIG_FILENAME: &str = "ferrflow.ts";

pub(crate) fn path_to_file_url(path: &Path) -> Result<String> {
    let canonical = path
        .canonicalize()
        .with_context(|| format!("Failed to resolve path: {}", path.display()))
        .error_code(error_code::CONFIG_RESOLVE_PATH)?;

    let path_str = canonical.to_string_lossy().to_string();

    let normalized = path_str
        .strip_prefix(r"\\?\")
        .unwrap_or(&path_str)
        .replace('\\', "/");

    let encoded = percent_encode_path(&normalized);

    if encoded.starts_with('/') {
        Ok(format!("file://{encoded}"))
    } else {
        Ok(format!("file:///{encoded}"))
    }
}

fn percent_encode_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for byte in path.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' | b':' => {
                out.push(byte as char);
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

const LOADER_SCRIPT: &str = r#"
const SHELL_SAFE_JS_ESCAPES = { '\\': '\\\\\\\\', "'": "\\'", '"': '\\"', '`': '\\`', '$': '\\$' };

function shellSafeJsString(value) {
  const body = String(value).replace(/[\\'"`$]/g, (ch) => SHELL_SAFE_JS_ESCAPES[ch]);
  return `'${body}'`;
}

function reifyHooks(hooks, fileUrl, runtime, hookPath) {
  if (!hooks || typeof hooks !== 'object') return hooks;
  const ctx = `{ package: process.env.FERRFLOW_PACKAGE, oldVersion: process.env.FERRFLOW_OLD_VERSION, newVersion: process.env.FERRFLOW_NEW_VERSION, bumpType: process.env.FERRFLOW_BUMP_TYPE, tag: process.env.FERRFLOW_TAG, dryRun: process.env.FERRFLOW_DRY_RUN === 'true', packagePath: process.env.FERRFLOW_PACKAGE_PATH, channel: process.env.FERRFLOW_CHANNEL || null, isPrerelease: process.env.FERRFLOW_IS_PRERELEASE === 'true', monorepo: process.env.FERRFLOW_MONOREPO === 'true', changelog: process.env.FERRFLOW_CHANGELOG || '', commits: JSON.parse(process.env.FERRFLOW_COMMITS_JSON || '[]'), bumpedFiles: JSON.parse(process.env.FERRFLOW_BUMPED_FILES_JSON || '[]'), allPackages: JSON.parse(process.env.FERRFLOW_ALL_PACKAGES_JSON || '[]'), releaseUrl: process.env.FERRFLOW_RELEASE_URL || null }`;
  const result = {};
  for (const [key, value] of Object.entries(hooks)) {
    if (typeof value === 'function') {
      const cmd = `${runtime} --input-type=module -e "const m = await import('${fileUrl}'); const cfg = typeof m.default === 'function' ? await m.default() : m.default; const hooks = ${hookPath}; await hooks[${shellSafeJsString(key)}](${ctx});"`;
      result[key] = cmd;
    } else {
      result[key] = value;
    }
  }
  return result;
}
"#;

fn loader_body(file_url: &str, runtime: &str) -> String {
    format!(
        r#"{LOADER_SCRIPT}
const m = await import('{file_url}');
const cfg = typeof m.default === 'function' ? await m.default() : m.default;
if (cfg.workspace && cfg.workspace.hooks) {{
  cfg.workspace.hooks = reifyHooks(cfg.workspace.hooks, '{file_url}', '{runtime}', 'cfg.workspace.hooks');
}}
if (cfg.package) {{
  for (const pkg of cfg.package) {{
    if (pkg.hooks) {{
      pkg.hooks = reifyHooks(pkg.hooks, '{file_url}', '{runtime}', 'cfg.package.find(p=>p.name===process.env.FERRFLOW_PACKAGE).hooks');
    }}
  }}
}}
process.stdout.write(JSON.stringify(cfg));"#
    )
}

pub(crate) fn load_js_ts_config(path: &Path) -> Result<Config> {
    use std::process::Command;

    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("ferrflow config");
    let file_url = path_to_file_url(path)?;

    let output = if ext == "ts" {
        let wrapper_tempdir = tempfile::tempdir()
            .with_context(|| "Failed to create temporary directory for TS loader")
            .error_code(error_code::CONFIG_WRITE_LOADER)?;
        let wrapper_path = wrapper_tempdir.path().join("loader.mjs");
        let tsx_available = Command::new("tsx").arg("--version").output().is_ok();
        let runtime = if tsx_available { "tsx" } else { "npx tsx" };

        let script = loader_body(&file_url, runtime);
        std::fs::write(&wrapper_path, &script)
            .with_context(|| "Failed to write temporary loader file")
            .error_code(error_code::CONFIG_WRITE_LOADER)?;

        let result = Command::new("tsx")
            .arg(&wrapper_path)
            .current_dir(wrapper_tempdir.path())
            .output()
            .or_else(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    Command::new("npx")
                        .args(["tsx"])
                        .arg(&wrapper_path)
                        .current_dir(wrapper_tempdir.path())
                        .output()
                } else {
                    Err(e)
                }
            })
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    anyhow::anyhow!(
                        "{filename} requires tsx but neither 'tsx' nor 'npx tsx' was found.\n\
                         Install with: npm install -g tsx"
                    )
                } else {
                    anyhow::anyhow!("Failed to execute tsx: {e}")
                }
            })
            .error_code(error_code::CONFIG_EVAL_TS);

        drop(wrapper_tempdir);
        result?
    } else {
        let script = loader_body(&file_url, "node");

        let parent = path.parent().unwrap_or(Path::new("."));
        Command::new("node")
            .args(["--input-type=module", "-e", &script])
            .current_dir(parent)
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    anyhow::anyhow!(
                        "{filename} requires Node.js but 'node' was not found in PATH.\n\
                         Install Node.js from https://nodejs.org/"
                    )
                } else {
                    anyhow::anyhow!("Failed to execute node: {e}")
                }
            })
            .error_code(error_code::CONFIG_EVAL_NODE)?
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(anyhow::anyhow!("Failed to evaluate {filename}:\n{stderr}"))
            .error_code(error_code::CONFIG_EVAL_FAILED)?;
    }

    let stdout = String::from_utf8(output.stdout)
        .with_context(|| format!("{filename} produced invalid UTF-8 output"))
        .error_code(error_code::CONFIG_INVALID_OUTPUT)?;

    let config = serde_json::from_str::<Config>(&stdout)
        .with_context(|| format!("{filename} did not produce valid JSON config"))
        .error_code(error_code::CONFIG_INVALID_JSON)?;
    if let Ok(value) = serde_json::from_str(&stdout) {
        super::unknown_keys::warn_unknown_keys::<Config>(value, path, &[]);
    }
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_with_shell_metacharacters_is_percent_encoded() {
        // The URL is interpolated into a single-quoted JS string that itself
        // sits in a double-quoted shell argument, so any of these reaching the
        // URL verbatim breaks out of one layer or the other.
        let encoded = percent_encode_path("/home/o'brien/a b/$HOME/`x`/\"q\"/repo");
        for ch in ['\'', '"', '$', '`', ' '] {
            assert!(!encoded.contains(ch), "{ch:?} survived encoding: {encoded}");
        }
        assert!(encoded.starts_with("/home/o%27brien/a%20b/%24HOME/"));
    }

    #[test]
    fn percent_encoding_round_trips_back_to_the_path() {
        let path = "/tmp/it's a $test/`b`/répertoire/ferrflow.js";
        let encoded = percent_encode_path(path);

        let mut decoded = Vec::new();
        let bytes = encoded.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'%' {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap();
                decoded.push(u8::from_str_radix(hex, 16).unwrap());
                i += 3;
            } else {
                decoded.push(bytes[i]);
                i += 1;
            }
        }
        assert_eq!(String::from_utf8(decoded).unwrap(), path);
    }

    #[test]
    fn the_loader_never_interpolates_a_package_name_into_the_script() {
        let script = loader_body("file:///tmp/ferrflow.js", "node");
        assert!(
            script.contains("p.name===process.env.FERRFLOW_PACKAGE"),
            "the package must be identified from the environment the hook runner \
             already sets, not spliced into the script: {script}"
        );
        assert!(
            !script.contains("pkg.name"),
            "a config-supplied package name is still reaching the script: {script}"
        );
    }

    #[test]
    fn a_hook_is_looked_up_by_key_rather_than_property_access() {
        let script = loader_body("file:///tmp/ferrflow.js", "node");
        assert!(
            script.contains("hooks[${shellSafeJsString(key)}]"),
            "a hook key that is not a bare identifier must still resolve: {script}"
        );
        assert!(!script.contains("hooks.${key}"), "{script}");
    }
}
