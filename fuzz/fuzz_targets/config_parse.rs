#![no_main]

use libfuzzer_sys::fuzz_target;

use ferrflow::config::{ConfigFileFormat, format_handler};

const FORMATS: &[ConfigFileFormat] = &[
    ConfigFileFormat::Json,
    ConfigFileFormat::Json5,
    ConfigFileFormat::Toml,
    ConfigFileFormat::Dotfile,
];

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };

    for format in FORMATS {
        let handler = format_handler(*format);
        let _ = handler.parse_value(text);

        let Ok(config) = handler.parse(text) else {
            continue;
        };
        let Ok(once) = handler.serialize(&config) else {
            continue;
        };
        let twice = handler
            .parse(&once)
            .and_then(|round| handler.serialize(&round))
            .unwrap_or_else(|e| panic!("{:?} cannot reparse what it serialized: {e}", format));

        assert_eq!(once, twice, "{:?} serialization is not stable", format);
    }
});
