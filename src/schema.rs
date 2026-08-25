use anyhow::{Context, Result};
use std::path::Path;

pub const BUNDLED_SCHEMA: &str = include_str!("../schema/ferrflow.json");

fn render(pretty: bool) -> Result<String> {
    let value: serde_json::Value = serde_json::from_str(BUNDLED_SCHEMA)
        .context("the bundled JSON schema is not valid JSON — the build artefact is corrupt")?;
    Ok(if pretty {
        serde_json::to_string_pretty(&value)?
    } else {
        serde_json::to_string(&value)?
    })
}

pub fn run(pretty: bool, output: Option<&Path>) -> Result<()> {
    let rendered = render(pretty)?;
    match output {
        Some(path) => {
            std::fs::write(path, format!("{rendered}\n"))
                .with_context(|| format!("failed to write schema to {}", path.display()))?;
            tracing::info!("Wrote JSON schema to {}", path.display());
        }
        None => println!("{rendered}"),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_schema_is_valid_json() {
        let value: serde_json::Value =
            serde_json::from_str(BUNDLED_SCHEMA).expect("bundled schema must parse");
        assert_eq!(value["$id"], "https://ferrflow.com/schema/ferrflow.json");
        assert!(value["properties"]["workspace"].is_object());
        assert!(value["properties"]["package"].is_object());
        assert!(value["properties"]["include"].is_object());
    }

    #[test]
    fn schema_covers_every_key_the_root_config_accepts() {
        let value: serde_json::Value =
            serde_json::from_str(BUNDLED_SCHEMA).expect("bundled schema must parse");
        let documented = value["properties"]
            .as_object()
            .expect("properties must be an object");

        assert!(
            value["additionalProperties"] == serde_json::Value::Bool(false),
            "the schema must stay strict, otherwise this test proves nothing"
        );
        for key in ["workspace", "include", "package"] {
            assert!(
                documented.contains_key(key),
                "`{key}` is accepted by Config but absent from the schema, so editors reject it"
            );
        }
    }

    #[test]
    fn bundled_schema_matches_the_checked_in_file() {
        let on_disk =
            std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/schema/ferrflow.json"))
                .expect("schema/ferrflow.json must exist");
        assert_eq!(
            BUNDLED_SCHEMA, on_disk,
            "the embedded schema drifted from schema/ferrflow.json — rebuild the binary"
        );
    }

    #[test]
    fn compact_is_single_line_and_pretty_is_multiline() {
        let compact = render(false).unwrap();
        let pretty = render(true).unwrap();
        assert!(serde_json::from_str::<serde_json::Value>(&compact).is_ok());
        assert!(serde_json::from_str::<serde_json::Value>(&pretty).is_ok());
        assert!(!compact.contains('\n'), "compact output must be one line");
        assert!(pretty.contains('\n'), "pretty output must be formatted");
    }
}
