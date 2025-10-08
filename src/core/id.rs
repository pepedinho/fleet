use rand::{RngCore, rngs::OsRng};

pub fn short_id() -> String {
    let mut bytes = [0u8; 8];
    OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)[..12].to_string()
}

pub fn format_commit(commit: &str) -> String {
    if commit.len() > 10 {
        commit[..10].to_string()
    } else {
        commit.to_string()
    }
}
