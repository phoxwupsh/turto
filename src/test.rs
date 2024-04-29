#[cfg(test)]
mod tests {

    use crate::utils::misc::ToEmoji;

    #[test]
    fn test_to_emoji() {
        assert_eq!(42.to_emoji(), "4️⃣2️⃣");
        assert_eq!(123.to_emoji(), "1️⃣2️⃣3️⃣");
        assert_eq!(56789.to_emoji(), "5️⃣6️⃣7️⃣8️⃣9️⃣");
    }
}