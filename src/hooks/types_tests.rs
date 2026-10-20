use std::collections::BTreeSet;

use regex::Regex;

use super::{HookCommit, HookFile, HookPackage};
use crate::config::CommitFormats;
use crate::config::loader_js::LOADER_SCRIPT;

const HOOK_CONTEXT_TYPES: &str = include_str!("../../npm/types/hook-context.d.ts");

fn interface_fields(name: &str) -> BTreeSet<String> {
    let block = Regex::new(&format!(r"(?s)export interface {name} \{{(.*?)\n\}}"))
        .unwrap()
        .captures(HOOK_CONTEXT_TYPES)
        .unwrap_or_else(|| panic!("interface {name} missing from hook-context.d.ts"))
        .get(1)
        .unwrap()
        .as_str()
        .to_string();
    Regex::new(r"(?m)^  (\w+)\??:")
        .unwrap()
        .captures_iter(&block)
        .map(|c| c[1].to_string())
        .collect()
}

fn serialized_keys(value: &impl serde::Serialize) -> BTreeSet<String> {
    serde_json::to_value(value)
        .unwrap()
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect()
}

fn loader_context_keys() -> BTreeSet<String> {
    let ctx = Regex::new(r"const ctx = `\{(.*?)\}`;")
        .unwrap()
        .captures(LOADER_SCRIPT)
        .expect("ctx literal missing from the loader script")[1]
        .to_string();
    Regex::new(r"(?:^|,)\s*(\w+):")
        .unwrap()
        .captures_iter(&ctx)
        .map(|c| c[1].to_string())
        .collect()
}

#[test]
fn hook_context_type_matches_the_object_the_loader_builds() {
    assert_eq!(loader_context_keys(), interface_fields("HookContext"));
}

#[test]
fn hook_commit_type_matches_the_serialized_commit() {
    let commit = HookCommit::from_commit(
        "abc1234",
        "feat(api): add endpoint",
        &CommitFormats::default(),
    );
    assert_eq!(serialized_keys(&commit), interface_fields("HookCommit"));
}

#[test]
fn hook_file_type_matches_the_serialized_file() {
    let file = HookFile {
        path: "Cargo.toml".into(),
        format: "toml".into(),
    };
    assert_eq!(serialized_keys(&file), interface_fields("HookFile"));
}

#[test]
fn hook_package_type_matches_the_serialized_package() {
    let package = HookPackage {
        name: "core".into(),
        version: "1.0.0".into(),
        bump: "minor".into(),
    };
    assert_eq!(serialized_keys(&package), interface_fields("HookPackage"));
}
