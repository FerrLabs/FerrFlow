#![no_main]

use libfuzzer_sys::fuzz_target;

use ferrflow::config::FileFormat;
use ferrflow::formats::get_handler;

const FORMATS: &[FileFormat] = &[
    FileFormat::Cabal,
    FileFormat::ChartYaml,
    FileFormat::Cmake,
    FileFormat::Csproj,
    FileFormat::GalaxyYaml,
    FileFormat::Gemspec,
    FileFormat::GoMod,
    FileFormat::Gradle,
    FileFormat::Helm,
    FileFormat::Json,
    FileFormat::MixExs,
    FileFormat::PackageSwift,
    FileFormat::PubspecYaml,
    FileFormat::Toml,
    FileFormat::Txt,
    FileFormat::Xml,
];

#[derive(arbitrary::Arbitrary, Debug)]
struct Input<'a> {
    format: u8,
    selector: Option<&'a str>,
    content: &'a [u8],
}

fuzz_target!(|input: Input| {
    let format = &FORMATS[input.format as usize % FORMATS.len()];
    let handler = get_handler(format);

    if let Ok(version) = handler.read_version_from_bytes(input.content, "fuzzed") {
        assert_eq!(
            version.trim(),
            version,
            "{format:?} returned a version with surrounding whitespace"
        );
    }

    if input.selector.is_some()
        && let Ok(version) =
            handler.read_version_from_bytes_with_selector(input.content, "fuzzed", input.selector)
    {
        assert_eq!(
            version.trim(),
            version,
            "{format:?} returned a selected version with surrounding whitespace"
        );
    }
});
