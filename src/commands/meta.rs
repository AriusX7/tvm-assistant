// Meta commands related to the bot directly are defined here.

use crate::{utils::constants::*, ConnectionPool};
use serenity::{
    framework::standard::{
        macros::{command, group},
        Args, CommandError, CommandResult,
    },
    model::prelude::*,
    prelude::*,
};

/// Sends [botname]'s invite url.
///
/// **Usage:** `[p]invite`
///
/// By using the invite url sent by the bot, the bot will get some
/// management permissions which are required for the bot to
/// function properly. You can review the permissions on the
/// invite page.
#[command]
async fn invite(ctx: &Context, msg: &Message) -> CommandResult {
    let user = ctx.cache.current_user().await;
    let invite_url = user
        .invite_url(&ctx.http, Permissions::from_bits_truncate(268494928))
        .await?;

    let embed_msg = &msg
        .channel_id
        .send_message(&ctx.http, |m| {
            m.embed(|e| {
                e.description(
                    format!(
                        "
                        Invite TvM Assistant to your bot by [clicking here]({}).\
                        \n\nInviting the bot will give it some management permissions. \
                        You can review them when you use the link.\
                        \n\nFor questions, suggestions and feedback, join [the support server]({}).
                        ",
                        invite_url, SUPPORT_SERVER
                    )
                    .trim(),
                );
                e.colour(EMBED_COLOUR);
                e.author(|a| {
                    a.name(format!("Invite {}", user.name));
                    a.icon_url(user.face());

                    a
                });

                e
            });

            m
        })
        .await;

    if embed_msg.is_err() {
        msg.channel_id
            .say(
                &ctx.http,
                format!(
                    "Invite TvM Assistant to your server using this link: <{}>\
                \n\nInviting the bot will give it some management permissions. \
                You can review them when you use the link.",
                    invite_url
                ),
            )
            .await?;
    }

    Ok(())
}

/// Shows info about [botname].
///
/// **Usage:** `[p]info`
///
/// Embed Links permission is required for this command to run. A lot of
/// other [botname] commands require Embed Links permission too.
#[command("info")]
async fn info_command(ctx: &Context, msg: &Message) -> CommandResult {
    let user = ctx.cache.current_user().await;
    let invite_url = user
        .invite_url(&ctx.http, Permissions::from_bits_truncate(268494928))
        .await?;

    let desc = concat!(
        "TvM Assistant is a Discord bot with utility commands to make hosting TvMs easier.",
        "\n\nSome of the bot features include:",
        "\n\n- Setup roles and channel creation",
        "\n- Management of sign-ups, sign-outs, spectators and replacements",
        "\n- In-built logging to detect and ignore private channels",
        "\n- Quick creation of player, mafia and spectator chats",
        "\n- Vote counts and time since day/night started",
        "\n- Richer text formatting",
    );

    let links = format!(
        "\n- [Invite to your server]({})\
        \n- [Support server]({})\
        \n- [Quickstart]({})\
        \n- [Commands Reference]({})\
        \n- [Source Code]({})",
        invite_url, SUPPORT_SERVER, QUICKSTART, COMMANDS_REFERENCE, SOURCE_CODE
    );

    let info_msg = msg
        .channel_id
        .send_message(&ctx.http, |m| {
            m.embed(|e| {
                e.description(desc);
                e.field("\u{200b}\nQuick Links", links, false);
                e.colour(EMBED_COLOUR);
                e.author(|a| {
                    a.name(format!("About {}", user.name));
                    a.icon_url(user.face());

                    a
                });

                e
            });

            m
        })
        .await;

    if info_msg.is_err() {
        msg.channel_id
            .say(&ctx.http, "I require the embed links permission.")
            .await?;
    }

    Ok(())
}

/// Sets custom prefix for the server.
///
/// **Usage:** `[p]setprefix <prefix>`
///
/// Prefix can be any valid unicode character or string, without space,
/// but it is recommended to keep it simple.
///
/// Only server administrators can use this command. A server can only have
/// one prefix at a time.
#[command("setprefix")]
#[num_args(1)]
#[required_permissions(ADMINISTRATOR)]
#[only_in("guilds")]
async fn set_prefix(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let prefix = args.message();

    let guild = match msg.guild(&ctx).await {
        Some(i) => i,
        None => return Err(CommandError::from("I couldn't fetch server details.")),
    };

    let data_read = ctx.data.read().await;
    let pool = data_read.get::<ConnectionPool>().unwrap();

    sqlx::query!(
        "
        INSERT INTO prefixes (
            guild_id, prefix
        ) VALUES (
            $1, $2
        ) ON CONFLICT (guild_id)
        DO UPDATE SET prefix = $2;
        ",
        guild.id.0 as i64,
        prefix.to_string()
    )
    .execute(pool)
    .await?;

    msg.channel_id
        .say(&ctx.http, format!("Updated server prefix to `{}`.", prefix))
        .await?;

    Ok(())
}

#[group("Miscellaneous")]
#[commands(invite, info_command, set_prefix)]
#[description("Meta commands related to the bot.")]
struct Misc;
