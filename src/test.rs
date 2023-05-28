#[cfg(test)]
mod tests {
    use crate::{
        utils::{convert_to_emoji},
    };

    #[test]
    fn test_convert_to_emoji() {
        assert_eq!(convert_to_emoji(42), "4️⃣2️⃣");
        assert_eq!(convert_to_emoji(123), "1️⃣2️⃣3️⃣");
        assert_eq!(convert_to_emoji(56789), "5️⃣6️⃣7️⃣8️⃣9️⃣");
    }
}