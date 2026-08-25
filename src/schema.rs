use anyhow::{Context, Result};
use std::path::Path;

pub const BUNDLED_SCHEMA: &str = include_str!("../schema/ferrflow.json");
pub const BUNDLED_PACKAGE_SCHEMA: &str = include_str!("../schema/ferrflow-package.json");

fn render(pretty: bool, package: bool) -> Result<String> {
    let source = if package {
        BUNDLED_PACKAGE_SCHEMA
    } else {
        BUNDLED_SCHEMA
    };
    let value: serde_json::Value = serde_json::from_str(source)
        .context("the bundled JSON schema is not valid JSON — the build artefact is corrupt")?;
    Ok(if pretty {
        serde_json::to_string_pretty(&value)?
    } else {
        serde_json::to_string(&value)?
    })
}

pub fn run(pretty: bool, package: bool, output: Option<&Path>) -> Result<()> {
    let rendered = render(pretty, package)?;
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
    fn package_schema_matches_the_checked_in_file() {
        let on_disk = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/schema/ferrflow-package.json"
        ))
        .expect("schema/ferrflow-package.json must exist");
        assert_eq!(
            BUNDLED_PACKAGE_SCHEMA, on_disk,
            "the embedded package schema drifted from schema/ferrflow-package.json — rebuild the binary"
        );
    }

    #[test]
    fn package_schema_describes_the_same_package_as_the_root_schema() {
        let root: serde_json::Value = serde_json::from_str(BUNDLED_SCHEMA).unwrap();
        let fragment: serde_json::Value = serde_json::from_str(BUNDLED_PACKAGE_SCHEMA).unwrap();

        let inline = root["properties"]["package"]["items"]["properties"]
            .as_object()
            .expect("the root schema must describe package items");
        let standalone = fragment["properties"]
            .as_object()
            .expect("the package schema must have properties");

        for (key, value) in inline {
            assert_eq!(
                standalone.get(key),
                Some(value),
                "`{key}` differs between the root schema and the package schema"
            );
        }
        for key in standalone.keys() {
            assert!(
                key == "$schema" || inline.contains_key(key),
                "`{key}` exists only in the package schema, so an included file accepts a key the root config does not"
            );
        }
    }

    #[test]
    fn package_schema_resolves_its_own_refs() {
        let root: serde_json::Value = serde_json::from_str(BUNDLED_SCHEMA).unwrap();
        let fragment: serde_json::Value = serde_json::from_str(BUNDLED_PACKAGE_SCHEMA).unwrap();

        assert_eq!(
            fragment["$defs"], root["$defs"],
            "the package schema copies $defs from the root schema, and the copy drifted"
        );

        let rendered = fragment.to_string();
        for reference in rendered.split("\"$ref\":").skip(1) {
            let target = reference
                .trim_start()
                .trim_start_matches('"')
                .split('"')
                .next()
                .unwrap();
            let name = target
                .strip_prefix("#/$defs/")
                .unwrap_or_else(|| panic!("{target} is not a local $defs reference"));
            assert!(
                fragment["$defs"].get(name).is_some(),
                "{target} does not resolve inside the package schema"
            );
        }
    }

    #[test]
    fn package_schema_makes_path_optional_but_still_demands_a_name() {
        let fragment: serde_json::Value = serde_json::from_str(BUNDLED_PACKAGE_SCHEMA).unwrap();
        let required: Vec<&str> = fragment["required"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();

        assert_eq!(required, vec!["name"]);
        assert!(fragment["properties"]["path"].is_object());
        assert_eq!(
            fragment["additionalProperties"],
            serde_json::Value::Bool(false)
        );
    }

    #[test]
    fn package_flag_emits_the_package_schema() {
        let rendered = render(false, true).unwrap();
        let value: serde_json::Value = serde_json::from_str(&rendered).unwrap();
        assert_eq!(
            value["$id"],
            "https://ferrflow.com/schema/ferrflow-package.json"
        );

        let rendered = render(false, false).unwrap();
        let value: serde_json::Value = serde_json::from_str(&rendered).unwrap();
        assert_eq!(value["$id"], "https://ferrflow.com/schema/ferrflow.json");
    }

    #[test]
    fn compact_is_single_line_and_pretty_is_multiline() {
        let compact = render(false, false).unwrap();
        let pretty = render(true, false).unwrap();
        assert!(serde_json::from_str::<serde_json::Value>(&compact).is_ok());
        assert!(serde_json::from_str::<serde_json::Value>(&pretty).is_ok());
        assert!(!compact.contains('\n'), "compact output must be one line");
        assert!(pretty.contains('\n'), "pretty output must be formatted");
    }
}
