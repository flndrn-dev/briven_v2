//! Reserved saved-copy timelines never have writable computes or credentials.
//! Restoring creates another timeline; the saved timeline itself stays untouched.
pub fn is_saved_copy(name: &str) -> bool {
    name.strip_prefix("sc-").is_some_and(|suffix| {
        suffix.len() == 24
            && suffix
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    })
}
