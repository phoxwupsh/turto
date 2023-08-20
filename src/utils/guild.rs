use serenity::model::prelude::{ChannelId, Guild, UserId};

pub trait GuildUtil {
    fn get_user_voice_channel(&self, user: &UserId) -> Option<ChannelId>;
}

impl GuildUtil for Guild {
    fn get_user_voice_channel(&self, user: &UserId) -> Option<ChannelId> {
        self
            .voice_states
            .get(user)
            .and_then(|voice_state| voice_state.channel_id)
    }
}