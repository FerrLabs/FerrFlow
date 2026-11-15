#![no_main]

use arbitrary::Arbitrary;
use ferrflow_api::bot::exchange_policy::{Denial, check_event, check_ref};
use ferrflow_api::bot::oidc_verify::{Audience, GhaClaims};
use libfuzzer_sys::fuzz_target;

const ALLOWED_EVENTS: [&str; 4] = ["push", "workflow_dispatch", "schedule", "release"];

#[derive(Debug, Arbitrary)]
struct Input {
    token_body: Vec<u8>,
    ref_: String,
    event_name: String,
    default_branch: String,
    suffix: String,
    needle: String,
    audiences: Vec<String>,
}

fn claims(ref_: String, event_name: String, aud: Audience) -> GhaClaims {
    GhaClaims {
        repository: "FerrLabs/FerrFlow".into(),
        repository_owner: "FerrLabs".into(),
        ref_,
        event_name,
        workflow: "Release".into(),
        run_id: "1".into(),
        actor: "octocat".into(),
        sub: "sub".into(),
        iss: "iss".into(),
        aud,
        exp: 0,
    }
}

fuzz_target!(|input: Input| {
    if let Ok(parsed) = serde_json::from_slice::<GhaClaims>(&input.token_body) {
        let _ = check_event(&parsed);
        let _ = check_ref(&parsed, &input.default_branch);
        let _ = parsed.aud.contains(&input.needle);
    }

    let verdict = check_event(&claims(
        String::new(),
        input.event_name.clone(),
        Audience::None,
    ));
    assert_eq!(
        verdict.is_ok(),
        ALLOWED_EVENTS.contains(&input.event_name.as_str())
    );

    let branch = format!("refs/heads/{}", input.default_branch);
    let on_default = claims(branch.clone(), String::new(), Audience::None);
    assert_eq!(check_ref(&on_default, &input.default_branch), Ok(()));

    if !input.suffix.is_empty() {
        let lookalike = claims(
            format!("{branch}{}", input.suffix),
            String::new(),
            Audience::None,
        );
        assert_eq!(
            check_ref(&lookalike, &input.default_branch),
            Err(Denial::Ref)
        );
    }

    let arbitrary_ref = claims(input.ref_.clone(), String::new(), Audience::None);
    if check_ref(&arbitrary_ref, &input.default_branch).is_ok() {
        assert!(input.ref_.starts_with("refs/tags/") || input.ref_ == branch);
    }

    let single = Audience::Single(input.needle.clone());
    assert!(single.contains(&input.needle));
    let many = Audience::Many(input.audiences.clone());
    assert_eq!(
        many.contains(&input.needle),
        input.audiences.contains(&input.needle)
    );
    assert!(!Audience::None.contains(&input.needle));
});
