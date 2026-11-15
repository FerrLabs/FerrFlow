//! Hosted FerrFlow bot — the GitHub Actions token service.
//!
//! A workflow using the FerrFlow Action requests a GitHub OIDC ID token for
//! audience `ferrflow.ferrlabs.com` and POSTs it to `/v1/ferrflow/token`.
//! We verify that JWT against GitHub's JWKS ([`oidc_verify`]), sign a short
//! App JWT with the FerrFlow App's RSA key ([`app_jwt`]), find the App's
//! installation on the caller's owner and mint a ~1 hour installation token
//! scoped to the single repository that asked ([`github_client`]). The runner
//! pushes tags and publishes releases with it — no PAT, no long-lived secret
//! at rest anywhere.

pub mod app_jwt;
pub mod exchange_policy;
pub mod github_client;
pub mod jwks;
pub mod oidc_verify;
