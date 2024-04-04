use sha256::digest;
use std::time::{SystemTime, UNIX_EPOCH};

pub trait ToEmoji {
    fn to_emoji(&self) -> String;
}

impl ToEmoji for usize {
    fn to_emoji(&self) -> String {
        self.to_string()
            .chars()
            .map(|ch| match ch {
                '0' => "0️⃣",
                '1' => "1️⃣",
                '2' => "2️⃣",
                '3' => "3️⃣",
                '4' => "4️⃣",
                '5' => "5️⃣",
                '6' => "6️⃣",
                '7' => "7️⃣",
                '8' => "8️⃣",
                '9' => "9️⃣",
                _ => unreachable!(),
            })
            .collect()
    }
}

pub fn sha256_now() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_be_bytes();
    digest(&now)
}
