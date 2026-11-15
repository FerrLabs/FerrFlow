use crate::routes::marketplace;

#[must_use]
pub fn webhook_signature_matches(secret: &str, body: &[u8], header: &str) -> bool {
    marketplace::signature_matches(secret, body, header)
}
