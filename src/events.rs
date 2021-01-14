// This module contains functions to handle message events fired by serenity.
//
// We only log message edits and deletions.

use crate::{
    utils::{
        converters::get_channel_from_id, message::get_jump_url,
    },
    ConnectionPool,
};
use chrono::Utc;
use serenity::{
    http::AttachmentType,
    model::{
        misc::Mentionable,
        prelude::{ChannelId, GuildChannel, Message, MessageId, MessageUpdateEvent, RoleId},
    },
    prelude::Context,
};
use serenity_utils::{prelude::EmbedBuilder, formatting::text_to_file};

use tracing::{error, instrument};

pub(crate) struct LogSettings {
    // `guild_id` exists here so we can use `*` in sql queries.
    // It is not intended to be read.
    #[allow(unused)]
    pub(crate) guild_id: i64,
    pub(crate) log_channel_id: Option<i64>,
    pub(crate) blacklist_channel_ids: Option<Vec<i64>>,
    pub(crate) whitelist_channel_ids: Option<Vec<i64>>,
}

#[instrument(skip(ctx))]
pub(crate) async fn message_update_handler(
    ctx: Context,
    old_if_available: Option<Message>,
    new: Option<Message>,
    event: MessageUpdateEvent,
) {
    // We can't compare messages if we don't get the old one.
    let old_content = match old_if_available {
        Some(m) => m.content,
        None => match event.content {
            Some(c) => c,
            None => {
                error!("I couldn't get old message content.");
                return;
            }
        },
    };
    let new = match new {
        Some(m) => m,
        None => return,
    };

    if new.content == old_content {
        return;
    }

    let guild = match new.guild(&ctx).await {
        Some(i) => i,
        None => return,
    };

    let data_read = ctx.data.read().await;
    let pool = data_read.get::<ConnectionPool>().unwrap();

    let settings: LogSettings = match sqlx::query_as!(
        LogSettings,
        "SELECT * FROM logging WHERE guild_id = $1;",
        guild.id.0 as i64
    )
    .fetch_one(pool)
    .await
    {
        Ok(r) => r,
        Err(_) => {
            error!("Unable to get logging channel from database.");
            return;
        }
    };

    let channel = match guild.channels.get(&new.channel_id) {
        Some(c) => c,
        None => return,
    };

    if !is_allowed_channel(&ctx, channel, &settings).await {
        return;
    }

    let log_channel = match get_channel_from_id(&ctx, guild.id, settings.log_channel_id).await {
        Ok(c) => c,
        Err(_) => return,
    };

    // All checks passed. We'll log the message now.
    let mut embed = EmbedBuilder::new();
    embed
        .set_colour(0xFF9300)
        .set_timestamp(Utc::now().to_rfc3339())
        .set_description(format!(
            "[Click here to jump to the message.]({})",
            get_jump_url(&new)
        ));

    let old_file = get_added_fields_and_file(&old_content, &mut embed, "Before");

    let new_file = get_added_fields_and_file(&new.content, &mut embed, "After");

    embed
        .add_field(("Channel", channel.mention(), false))
        .set_footer_with(|f| f.set_text(format!("Message ID: {}", new.id.0)))
        .set_author_with(|a| a.set_name(format!(
                "{} ({}) - Edited Message",
                new.author.tag(),
                new.author.id.0
            ))
            .set_icon_url(new.author.face())
        );

    let files = vec![old_file, new_file].into_iter().flatten();

    let msg = &log_channel
        .send_message(&ctx.http, |m| m.set_embed(embed.to_create_embed()))
        .await;

    // Add files in a separate message, so that files are shown after embed.
    let _ = log_channel.send_files(&ctx.http, files, |f| f).await;

    if let Err(why) = msg {
        error!("Failed to log message edit: {}", why);
    }
}

pub(crate) async fn message_delete_handler(
    ctx: Context,
    channel_id: ChannelId,
    deleted_message_id: MessageId,
) {
    match ctx.cache.message(channel_id, deleted_message_id).await {
        Some(m) => cached_message_handler(&ctx, &m).await,
        None => uncached_message_handler(&ctx, channel_id, deleted_message_id).await,
    };
}

#[instrument(skip(ctx))]
pub(crate) async fn message_delete_bulk_handler(
    ctx: Context,
    channel_id: ChannelId,
    message_ids: Vec<MessageId>,
) {
    // This shouldn't happen, but doesn't hurt to add this check.
    if message_ids.is_empty() {
        return;
    }

    let channel = match channel_id.to_channel(&ctx.http).await {
        Ok(c) => {
            if let Some(gc) = c.guild() {
                gc
            } else {
                return;
            }
        }
        Err(_) => {
            error!("Failed to retrieve channel while logging bulk delete.");
            return;
        }
    };

    // We can be fine with just the ID.
    let guild_id = channel.guild_id;

    let data_read = ctx.data.read().await;
    let pool = data_read.get::<ConnectionPool>().unwrap();

    let settings: LogSettings = match sqlx::query_as!(
        LogSettings,
        "SELECT * FROM logging WHERE guild_id = $1;",
        guild_id.0 as i64
    )
    .fetch_one(pool)
    .await
    {
        Ok(r) => r,
        Err(_) => {
            error!("Unable to get logging channel from database.");
            return;
        }
    };

    if !is_allowed_channel(&ctx, &channel, &settings).await {
        return;
    }

    let log_channel = match get_channel_from_id(&ctx, guild_id, settings.log_channel_id).await {
        Ok(c) => c,
        Err(_) => return,
    };

    // All checks passed. Since we only store 10 messages in the cache, we'll just log the count.
    let msg = log_channel
        .send_message(&ctx.http, |m| {
            m.embed(|e| {
                e.colour(0xFF0000);
                e.description(format!(
                    "{} messages were deleted at once.",
                    message_ids.len()
                ));
                e.timestamp(Utc::now().to_rfc3339());
                e.field("Channel", channel.mention(), true);
                e.author(|a| {
                    a.name(format!("Deleted {} Message", message_ids.len()));

                    a
                });

                e
            });

            m
        })
        .await;

    if let Err(why) = msg {
        error!("Failed to log bulk message delete: {}", why);
    }
}

#[instrument(skip(ctx))]
async fn cached_message_handler(ctx: &Context, message: &Message) {
    let content = &message.content;

    if message.author.bot {
        return;
    }

    let guild = match message.guild(&ctx).await {
        Some(i) => i,
        None => return,
    };

    let data_read = ctx.data.read().await;
    let pool = data_read.get::<ConnectionPool>().unwrap();

    let settings: LogSettings = match sqlx::query_as!(
        LogSettings,
        "SELECT * FROM logging WHERE guild_id = $1;",
        guild.id.0 as i64
    )
    .fetch_one(pool)
    .await
    {
        Ok(r) => r,
        Err(_) => {
            error!("Unable to get logging channel from database.");
            return;
        }
    };

    let channel = match guild.channels.get(&message.channel_id) {
        Some(c) => c,
        None => return,
    };

    if !is_allowed_channel(&ctx, channel, &settings).await {
        return;
    }

    let log_channel = match get_channel_from_id(&ctx, guild.id, settings.log_channel_id).await {
        Ok(c) => c,
        Err(_) => return,
    };

    // Checks passed, we'll create embed now.
    // We'll check if we can get details of the user who deleted the message.
    let mut perp = None;
    // `action_type` is `72` for message deletes.
    if let Ok(logs) = guild
        .audit_logs(&ctx.http, Some(72), None, None, Some(5))
        .await
    {
        for (_, entry) in logs.entries {
            if let Some(options) = entry.options {
                if options.channel_id.unwrap_or(ChannelId(0)) == message.channel_id
                    && entry.target_id.unwrap_or(0) == message.author.id.0
                {
                    // `ok` will convert `Result` into `Option`.
                    perp = entry.user_id.to_user(&ctx.http).await.ok();
                }
            }
        }
    }

    let mut embed = EmbedBuilder::new();
    embed
        .set_description(&content)
        .set_colour(0xFF0000)
        .set_timestamp(Utc::now().to_rfc3339())
        .add_field(("Channel", channel.mention(), true));

    if let Some(user) = perp {
        embed.add_field((
            "Deleted By",
            format!("{} ({})", user.tag(), user.id.0),
            true,
        ));
    }
    // Attachments are generally not allowed in TvMs, so we can be fine with just listing
    // file names instead of attaching the attachments in the log message.
    // let mut files = String::new();
    if !message.attachments.is_empty() {
        let file_names = message
            .attachments
            .iter()
            .map(|a| a.filename.clone())
            .collect::<Vec<String>>()
            .join(", ");

        embed.add_field(("Attachments", file_names, true));
    }
    embed
        .set_footer_with(|f| f.set_text(format!(
            "Author ID: {}",
            message.author.id.0
        )))
        .set_author_with(|a| a.set_name(format!(
                "{} ({}) - Deleted Message",
                message.author.tag(),
                message.author.id.0
            ))
            .set_icon_url(message.author.face()),
        );

    let msg = &log_channel
        .send_message(&ctx.http, |m| m.set_embed(embed.to_create_embed()))
        .await;

    if let Err(why) = msg {
        error!("Failed to log message delete: {}", why);
    }
}

#[instrument(skip(ctx))]
async fn uncached_message_handler(ctx: &Context, channel_id: ChannelId, message_id: MessageId) {
    let channel = match channel_id.to_channel(&ctx.http).await {
        Ok(c) => {
            if let Some(gc) = c.guild() {
                gc
            } else {
                return;
            }
        }
        Err(_) => {
            error!("Failed to retrieve channel while logging uncached message delete.");
            return;
        }
    };

    // We can be fine with just the ID.
    let guild_id = channel.guild_id;

    let data_read = ctx.data.read().await;
    let pool = data_read.get::<ConnectionPool>().unwrap();

    let settings: LogSettings = match sqlx::query_as!(
        LogSettings,
        "SELECT * FROM logging WHERE guild_id = $1;",
        guild_id.0 as i64
    )
    .fetch_one(pool)
    .await
    {
        Ok(r) => r,
        Err(_) => {
            error!("Unable to get logging channel from database.");
            return;
        }
    };

    if !is_allowed_channel(&ctx, &channel, &settings).await {
        return;
    }

    let log_channel = match get_channel_from_id(&ctx, guild_id, settings.log_channel_id).await {
        Ok(c) => c,
        Err(_) => return,
    };

    // All checks passed, let's go.
    // Instead of creating an `Embed`, we'll create `CreateEmbed` directly.
    let msg = log_channel
        .send_message(&ctx.http, |m| {
            m.embed(|e| {
                e.colour(0xFF0000);
                e.description("Message's content is unknown.");
                e.timestamp(Utc::now().to_rfc3339());
                e.field("Channel", channel.mention(), true);
                e.author(|a| {
                    a.name("Deleted Message");

                    a
                });
                e.footer(|f| {
                    f.text(format!("Message ID: {}", message_id.0));

                    f
                });

                e
            });

            m
        })
        .await;

    if let Err(why) = msg {
        error!("Failed to log uncached message: {}", why);
    }
}

fn get_added_fields_and_file<'a>(
    content: &'a str,
    embed: &mut EmbedBuilder,
    iden: &'a str,
) -> Option<AttachmentType<'a>> {
    if content.len() > 1024 {
        let text = format!(
            "{}...\n\nFull message attached below.",
            content[..500].trim()
        );
        embed.add_field((format!("{} Content", iden), text, true));

        Some(text_to_file(&content, Some(format!("{}.txt", iden.to_ascii_lowercase())), false))
    } else {
        embed.add_field((format!("{} Content", iden), &content, true));

        None
    }
}

pub(crate) async fn is_allowed_channel(
    ctx: &Context,
    channel: &GuildChannel,
    settings: &LogSettings,
) -> bool {
    let whitelist_ids = match &settings.whitelist_channel_ids {
        Some(v) => v.clone(),
        None => Vec::new(),
    };

    // Whitelist takes precendence over everything else.
    // If a channel ID is in whitelist, we log message in that channel.
    if whitelist_ids.contains(&(channel.id.0 as i64)) {
        return true;
    }

    let blacklist_ids = match &settings.blacklist_channel_ids {
        Some(v) => v.clone(),
        None => Vec::new(),
    };

    // Blackist takes precendence after whitelist.
    // If a channel ID is in blacklist but not whitelist,
    // we don't log message in that channel.
    if blacklist_ids.contains(&(channel.id.0 as i64)) {
        return false;
    }

    // If a channel is not in the whitelist or the blacklist, we'll
    // check if it is a public channel. Public channels are logged.
    if let Ok(perms) = channel
        .permissions_for_role(&ctx.cache, RoleId(channel.guild_id.0))
        .await
    {
        perms.read_messages()
    } else {
        false
    }
}
