// This module contains commands related to message logging.
// Message related events fired by serenity are handled in `/src/events.rs`.

use crate::{
    events::{is_allowed_channel, LogSettings},
    utils::{
        checks::*,
        converters::{get_channel, get_channel_from_id},
    },
    ConnectionPool,
};
use serenity::{
    framework::standard::{
        macros::{command, group},
        Args, CommandError, CommandResult,
    },
    model::{misc::Mentionable, prelude::*},
    prelude::Context,
};
use std::fmt::Write;

/// Sets the message logging channel.
///
/// **Usage:** `[p]log channel [channel]`
///
/// If you don't supply a channel, a new channel named `log` will be created.
/// The channel will have the following permissions:
/// - `everyone`: Can read messages, cannot send messages or add reactions
/// - `me`: Can read messages, send messages, embed links and attach files
///
/// If you supply a channel when using the command, ensure that I have the above
/// four permissions. The first three are required for logging to work, the last one
/// is highly recommended, as messages with over 1024 characters will be sent as
/// files, and without the attach files permission, I will not be able to send a file.
#[command("channel")]
async fn log_channel(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let guild = match msg.guild(&ctx).await {
        Some(i) => i,
        None => return Err(CommandError::from("I couldn't fetch server details.")),
    };

    let input = args.message().to_string();

    let channel = match get_channel(&ctx, guild.id, Some(&input)).await {
        Ok(c) => c,
        Err(_) => {
            // let me = &ctx.cache.us
            let perms = vec![
                PermissionOverwrite {
                    allow: Permissions::READ_MESSAGES,
                    deny: Permissions::SEND_MESSAGES | Permissions::ADD_REACTIONS,
                    kind: PermissionOverwriteType::Role(RoleId(guild.id.0)),
                },
                PermissionOverwrite {
                    allow: Permissions::SEND_MESSAGES | Permissions::EMBED_LINKS,
                    deny: Permissions::empty(),
                    kind: PermissionOverwriteType::Member(ctx.cache.current_user().await.id),
                },
            ];
            match guild
                .create_channel(&ctx.http, |f| {
                    f.name("log").kind(ChannelType::Text).permissions(perms)
                })
                .await
            {
                Ok(c) => c,
                Err(_) => {
                    msg.channel_id
                        .say(
                            &ctx.http,
                            format!(
                                "No channel found from `{}`. I also couldn't create a new channel.",
                                input
                            ),
                        )
                        .await?;
                    return Ok(());
                }
            }
        }
    };

    let data_read = ctx.data.read().await;
    let pool = data_read.get::<ConnectionPool>().unwrap();

    sqlx::query!(
        "
        INSERT INTO logging (
            guild_id, log_channel_id
        ) VALUES (
            $1, $2
        ) ON CONFLICT (guild_id)
        DO UPDATE SET log_channel_id = $2;
        ",
        guild.id.0 as i64,
        channel.id.0 as i64
    )
    .execute(pool)
    .await?;

    msg.channel_id
        .say(
            &ctx.http,
            format!("Set {} as the logging channel.", channel.mention()),
        )
        .await?;

    Ok(())
}

/// Adds the specified channel to the log whitelist.
///
/// Usage: `[p]log whitelist <channel>`
///
/// The `whitelist` takes precendence over everything. If a channel is in the whitelist,
/// the messages sent it in will be logged as long as I am able to view messages in the
/// channel.
///
/// Don't add a channel to the blacklist to remove it from the whitelist. Use the
/// `[p]rwhitelist <channel>` command.
#[command("whitelist")]
#[min_args(1)]
async fn whitelist_channel(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let guild = match msg.guild(&ctx).await {
        Some(i) => i,
        None => return Err(CommandError::from("I couldn't fetch server details.")),
    };

    let input = args.message().to_string();

    let channel = match get_channel(&ctx, guild.id, Some(&input)).await {
        Ok(c) => c,
        Err(_) => {
            msg.channel_id
                .say(&ctx.http, format!("No channel found from `{}`.", input))
                .await?;
            return Ok(());
        }
    };

    let data_read = ctx.data.read().await;
    let pool = data_read.get::<ConnectionPool>().unwrap();

    sqlx::query!(
        "
        INSERT INTO logging (
            guild_id, whitelist_channel_ids
        ) VALUES (
            $1, array[$2]::bigint[]
        ) ON CONFLICT (guild_id)
        DO UPDATE SET whitelist_channel_ids = array_append(logging.whitelist_channel_ids, $2)
        WHERE logging.whitelist_channel_ids IS NULL
        OR not(logging.whitelist_channel_ids @> array[$2]::bigint[]);
        ",
        guild.id.0 as i64,
        channel.id.0 as i64
    )
    .execute(pool)
    .await?;

    msg.channel_id
        .say(
            &ctx.http,
            format!("Added {} to logging whitelist.", channel.mention()),
        )
        .await?;

    Ok(())
}

/// Adds the specified channel to the log blacklist.
///
/// Usage: `[p]log whitelist <channel>`
///
/// The `whitelist` takes precendence over the blacklist. If a channel is in the whitelist,
/// the messages sent it in will be logged even if it is in the blacklist (as long as I am
/// able to view messages in the channel).
///
/// Don't add a channel to the whitelist to remove it from the blacklist. Use the
/// `[p]rblacklist <channel>` command.
#[command("blacklist")]
#[min_args(1)]
async fn blacklist_channel(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let guild = match msg.guild(&ctx).await {
        Some(i) => i,
        None => return Err(CommandError::from("I couldn't fetch server details.")),
    };

    let input = args.message().to_string();

    let channel = match get_channel(&ctx, guild.id, Some(&input)).await {
        Ok(c) => c,
        Err(_) => {
            msg.channel_id
                .say(&ctx.http, format!("No channel found from `{}`.", input))
                .await?;
            return Ok(());
        }
    };

    let data_read = ctx.data.read().await;
    let pool = data_read.get::<ConnectionPool>().unwrap();

    sqlx::query!(
        "
        INSERT INTO logging (
            guild_id, blacklist_channel_ids
        ) VALUES (
            $1, array[$2]::bigint[]
        ) ON CONFLICT (guild_id)
        DO UPDATE SET blacklist_channel_ids = array_append(logging.blacklist_channel_ids, $2)
        WHERE logging.blacklist_channel_ids IS NULL
        OR not(logging.blacklist_channel_ids @> array[$2]::bigint[]);
        ",
        guild.id.0 as i64,
        channel.id.0 as i64
    )
    .execute(pool)
    .await?;

    msg.channel_id
        .say(
            &ctx.http,
            format!("Added {} to logging blacklist.", channel.mention()),
        )
        .await?;

    Ok(())
}

/// Removes the specified channel from the log whitelist.
///
/// Usage: `[p]log rwhitelist <channel>`
///
/// Don't add a channel to the blacklist to remove it from the whitelist. Use this command.
#[command("rwhitelist")]
#[min_args(1)]
async fn remove_whitelist_channel(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let guild = match msg.guild(&ctx).await {
        Some(i) => i,
        None => return Err(CommandError::from("I couldn't fetch server details.")),
    };

    let input = args.message().to_string();

    let channel = match get_channel(&ctx, guild.id, Some(&input)).await {
        Ok(c) => c,
        Err(_) => {
            msg.channel_id
                .say(&ctx.http, format!("No channel found from `{}`.", input))
                .await?;
            return Ok(());
        }
    };

    let data_read = ctx.data.read().await;
    let pool = data_read.get::<ConnectionPool>().unwrap();

    // This query removes all the occurences of the id.
    sqlx::query!(
        "
        UPDATE logging SET whitelist_channel_ids = array_remove(whitelist_channel_ids, $2)
        WHERE guild_id = $1;
        ",
        guild.id.0 as i64,
        channel.id.0 as i64
    )
    .execute(pool)
    .await?;

    msg.channel_id
        .say(
            &ctx.http,
            format!("Removed {} from logging whitelist.", channel.mention()),
        )
        .await?;

    Ok(())
}

/// Removes the specified channel from the log blacklist.
///
/// Usage: `[p]log rblacklist <channel>`
///
/// Don't add a channel to the whitelist to remove it from the blacklist. Use this command.
#[command("rblacklist")]
#[min_args(1)]
async fn remove_blacklist_channel(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let guild = match msg.guild(&ctx).await {
        Some(i) => i,
        None => return Err(CommandError::from("I couldn't fetch server details.")),
    };

    let input = args.message().to_string();

    let channel = match get_channel(&ctx, guild.id, Some(&input)).await {
        Ok(c) => c,
        Err(_) => {
            msg.channel_id
                .say(&ctx.http, format!("No channel found from `{}`.", input))
                .await?;
            return Ok(());
        }
    };

    let data_read = ctx.data.read().await;
    let pool = data_read.get::<ConnectionPool>().unwrap();

    // This query removes all the occurences of the id.
    sqlx::query!(
        "
        UPDATE logging SET blacklist_channel_ids = array_remove(blacklist_channel_ids, $2)
        WHERE guild_id = $1;
        ",
        guild.id.0 as i64,
        channel.id.0 as i64
    )
    .execute(pool)
    .await?;

    msg.channel_id
        .say(
            &ctx.http,
            format!("Removed {} from logging blacklist.", channel.mention()),
        )
        .await?;

    Ok(())
}

/// Shows the message log settings for this server.
///
/// Usage: `[p]log settings`
#[command("settings")]
async fn log_settings(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = match msg.guild(&ctx).await {
        Some(i) => i,
        None => return Err(CommandError::from("I couldn't fetch server details.")),
    };

    let data_read = ctx.data.read().await;
    let pool = data_read.get::<ConnectionPool>().unwrap();

    // This query removes all the occurences of the id.
    let settings: LogSettings = match sqlx::query_as_unchecked!(
        LogSettings,
        "SELECT * FROM logging WHERE guild_id = $1;",
        guild.id.0 as i64
    )
    .fetch_one(pool)
    .await
    {
        Ok(r) => r,
        Err(_) => {
            return Err(CommandError::from(
                "Unable to get logging channel from database.",
            ))
        }
    };

    let whitelist = match &settings.whitelist_channel_ids {
        Some(ids) => ids.clone(),
        None => Vec::new(),
    };

    let blacklist = match &settings.blacklist_channel_ids {
        Some(ids) => ids.clone(),
        None => Vec::new(),
    };

    let mut allowed = String::new();
    for (_, channel) in guild.channels {
        match channel.kind {
            ChannelType::Text => (),
            _ => continue,
        }

        if !whitelist.contains(&(channel.id.0 as i64))
            && !blacklist.contains(&(channel.id.0 as i64))
            && is_allowed_channel(&ctx, &channel, &settings).await
        {
            let _ = write!(allowed, "\n{}", channel.mention());
        }
    }

    if allowed.is_empty() {
        allowed = "No channels".to_string();
    }

    let log_channel_str = match get_channel_from_id(&ctx, guild.id, settings.log_channel_id).await {
        Ok(c) => format!("Log Channel: {}", c.mention()),
        Err(_) => String::from("Log channel not set!"),
    };

    let whitelist_str = if !whitelist.is_empty() {
        let mut text = String::new();
        for id in whitelist {
            let _ = write!(text, "\n<#{}>", id);
        }
        text
    } else {
        String::from("No channels")
    };

    let blacklist_str = if !blacklist.is_empty() {
        let mut text = String::new();
        for id in blacklist {
            let _ = write!(text, "\n<#{}>", id);
        }
        text
    } else {
        String::from("No channels")
    };

    let sent = msg
        .channel_id
        .send_message(&ctx.http, |m| {
            m.embed(|e| {
                e.title("Log Settings");
                e.description(log_channel_str);
                e.colour(0x00CDFF);
                e.fields(vec![
                    ("Whitelisted Channels", whitelist_str, false),
                    ("Blacklisted Channels", blacklist_str, false),
                    ("Default Allowed Channels", allowed, false),
                ]);

                e
            });

            m
        })
        .await;

    if sent.is_err() {
        msg.channel_id
            .say(&ctx.http, "I couldn't send the log settings message.")
            .await?;
    }

    Ok(())
}

#[group]
#[description = "Commands related to message logging."]
#[prefix("log")]
#[checks("is_host_or_admin")]
#[only_in("guilds")]
#[commands(
    log_channel,
    whitelist_channel,
    blacklist_channel,
    remove_whitelist_channel,
    remove_blacklist_channel,
    log_settings
)]
#[default_command(log_settings)]
struct Logging;
