use serenity::model::prelude::{ChannelId, Guild, UserId};

pub trait GuildUtil {
    fn get_user_voice_channel(&self, user: &UserId) -> Option<ChannelId>;
    fn cmp_voice_channel(&self, first_user: &UserId, second_user: &UserId) -> VoiceChannelState;
}

impl GuildUtil for Guild {
    fn get_user_voice_channel(&self, user: &UserId) -> Option<ChannelId> {
        self.voice_states
            .get(user)
            .and_then(|voice_state| voice_state.channel_id)
    }

    fn cmp_voice_channel(&self, first_user: &UserId, second_user: &UserId) -> VoiceChannelState {
        let first = self.get_user_voice_channel(first_user);
        let second = self.get_user_voice_channel(second_user);
        match (first, second) {
            (None, None) => VoiceChannelState::None,
            (Some(first_voice_channel), None) => VoiceChannelState::OnlyFirst(first_voice_channel),
            (None, Some(second_voice_channel)) => VoiceChannelState::OnlySecond(second_voice_channel),
            (Some(first_voice_channel), Some(second_voice_channel)) => {
                if first_voice_channel == second_voice_channel {
                    VoiceChannelState::Same(first_voice_channel)
                } else {
                    VoiceChannelState::Different(first_voice_channel, second_voice_channel)
                }
            }
        }
    }
}

pub enum VoiceChannelState {
    Same(ChannelId),
    Different(ChannelId, ChannelId),
    OnlyFirst(ChannelId),
    OnlySecond(ChannelId),
    None
}
