use rand::{RngCore, rngs::OsRng};

pub fn short_id() -> String {
    let mut bytes = [0u8; 8];
    OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)[..12].to_string()
}
