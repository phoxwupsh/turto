use sha256::digest;

pub trait ToEmoji {
    fn to_emoji(&self) -> String;
}

impl ToEmoji for usize {
    fn to_emoji(&self) -> String {
        let s = self.to_string();
        let mut result = String::with_capacity(s.len() * 7);
        for c in s.chars() {
            result.push_str(match c {
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
                _ => unreachable!(), // since input is guaranteed to be a digit
            })
        }
        result
    }
}

pub fn sha256_now() -> String {
    let now = time::UtcDateTime::now().unix_timestamp().to_le_bytes();
    digest(&now)
}

#[cfg(test)]
mod test {
    use super::ToEmoji;

    #[test]
    fn test_to_emoji() {
        assert_eq!(42.to_emoji(), "4️⃣2️⃣");
        assert_eq!(123.to_emoji(), "1️⃣2️⃣3️⃣");
        assert_eq!(56789.to_emoji(), "5️⃣6️⃣7️⃣8️⃣9️⃣");
    }
}
