use serenity::model::prelude::Message;

pub(crate) fn get_jump_url(msg: &Message) -> String {
    if let Some(guild_id) = msg.guild_id {
        format!("https://discord.com/channels/{}/{}/{}", guild_id.0, msg.channel_id.0, msg.id.0)
    } else {
        format!("https://discord.com/channels/@me/{}/{}", msg.channel_id.0, msg.id.0)
    }
}
