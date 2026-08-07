//! Stable id generation and validation.

/// Returns true if `id` is non-empty and contains only safe characters
/// (`[A-Za-z0-9_-]`). Used to keep ids usable as DB keys and in URLs.
pub fn validate_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Generates a fresh prop id like `prop_0042`. Not unique by itself — the
/// caller is responsible for ensuring no collision with existing ids.
pub fn make_prop_id(sequence: u32) -> String {
    format!("prop_{sequence:04}")
}
