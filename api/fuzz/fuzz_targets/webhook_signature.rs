#![no_main]

use arbitrary::Arbitrary;
use ferrflow_api::fuzzing::webhook_signature_matches;
use hmac::{Hmac, KeyInit, Mac};
use libfuzzer_sys::fuzz_target;
use sha2::Sha256;

#[derive(Debug, Arbitrary)]
struct Input {
    secret: String,
    body: Vec<u8>,
    header: String,
    flip: usize,
}

fn sign(secret: &str, body: &[u8]) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("any key length");
    mac.update(body);
    format!("sha256={}", hex::encode(mac.finalize().into_bytes()))
}

fuzz_target!(|input: Input| {
    let _ = webhook_signature_matches(&input.secret, &input.body, &input.header);

    let signature = sign(&input.secret, &input.body);
    assert!(webhook_signature_matches(
        &input.secret,
        &input.body,
        &signature
    ));
    assert!(webhook_signature_matches(
        &input.secret,
        &input.body,
        &signature.to_uppercase().replacen("SHA256=", "sha256=", 1),
    ));

    if !input.body.is_empty() {
        let mut tampered = input.body.clone();
        let at = input.flip % tampered.len();
        tampered[at] ^= 1;
        assert!(!webhook_signature_matches(
            &input.secret,
            &tampered,
            &signature
        ));
    }

    let truncated = &signature[..signature.len() - 2];
    assert!(!webhook_signature_matches(
        &input.secret,
        &input.body,
        truncated
    ));
});
