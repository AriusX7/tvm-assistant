//! Owner-only commands are defined in this module.

use crate::ShardManagerContainer;
use serenity::{
    framework::standard::{
        macros::{command, group},
        CommandResult,
    },
    model::prelude::*,
    prelude::*,
};
use std::fmt::Write;

/// The bot tries to shutdown gracefully.
///
/// **Usage:** `[p]quit`
///
/// **Alias:** `shutdown`
#[command]
#[aliases("shutdown")]
async fn quit(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;

    if let Some(manager) = data.get::<ShardManagerContainer>() {
        msg.reply(ctx, "Shutting down!").await?;
        manager.lock().await.shutdown_all().await;
    } else {
        msg.reply(ctx, "There was a problem getting the shard manager")
            .await?;

        return Ok(());
    }

    Ok(())
}

/// Lists all servers the bot is in.
///
/// **Usage:** `[p]servers`
#[command]
async fn servers(ctx: &Context, msg: &Message) -> CommandResult {
    let guilds = ctx.cache.current_user().await.guilds(&ctx.http).await;

    let mut guilds_str = String::new();
    if let Ok(guilds) = guilds {
        for (idx, guild) in guilds.into_iter().enumerate() {
            write!(guilds_str, "\n{}. {}", idx + 1, guild.name)?;
        }
    }

    msg.channel_id.say(&ctx.http, guilds_str.trim()).await?;

    Ok(())
}

#[group]
#[help_available(false)]
#[commands(quit, servers)]
#[owners_only]
#[description("Owner-only commands.")]
struct Owner;
