pub trait ToEmoji {
    fn to_emoji(&self) -> String;
}

impl ToEmoji for i32 {
    fn to_emoji(&self) -> String {
        let num_str = self.to_string();
        let mut emoji_str = String::new();

        if self < &0 {
            emoji_str.push('➖');
        }

        for ch in num_str.chars() {
            let emoji = match ch {
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
                _ => continue,
            };
            emoji_str.push_str(emoji);
        }
        emoji_str
    }
}