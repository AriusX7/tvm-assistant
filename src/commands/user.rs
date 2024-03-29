//! This module contains commands and related functiosn for general
//! users, like players, spectators and replacements.

use crate::{
    commands::{
        host::{get_na_channel, CycleContainer, Data},
        setup::Cycle,
    },
    utils::{
        constants::EMBED_COLOUR,
        converters::{get_channel, get_channel_from_id, get_member, get_role, to_channel, to_role},
        formatting::{capitalize, clean_user_mentions, markdown_to_files},
        message::get_jump_url_with_guild,
        tos,
    },
    ConnectionPool, RequestClient,
};
use chrono::{offset::Utc, Datelike, Duration};
use indexmap::IndexMap;
use once_cell::sync::Lazy;
use regex::Regex;
use serenity::{
    framework::standard::{
        macros::{command, group},
        Args, CommandError, CommandResult,
    },
    futures::StreamExt,
    model::{
        misc::Mentionable,
        prelude::{
            Guild, GuildChannel, GuildId, Member, Message, PermissionOverwriteType, Role, User,
        },
    },
    prelude::Context,
    utils::{content_safe, ContentSafeOptions},
};
use sqlx::types::Json;
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    fmt::Write,
    fs,
};
use tracing::error;

struct SignSettings {
    cycle: Json<Cycle>,
    signups_on: Option<bool>,
    total_players: Option<i16>,
    total_signups: Option<i16>,
    signups_channel_id: Option<i64>,
    player_role_id: Option<i64>,
    spec_role_id: Option<i64>,
    repl_role_id: Option<i64>,
}

enum Roles {
    Player,
    Spectator,
    Replacement,
}

#[derive(Debug, PartialEq, Eq, Hash)]
enum Vote {
    Vtl(String),
    UnVtl(String),
    Vtnl,
}

struct VoteData {
    pub(crate) player_role_id: Option<i64>,
    pub(crate) cycle: Option<Json<Cycle>>,
    pub(crate) players: Option<Vec<i64>>,
}

/// Sign-in for the TvM.
///
/// **Usage:** `[p]in`
///
/// The command must be used in the sign-ups channel. It cannot be used
/// once the game has started.
#[command("in")]
async fn sign_in(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = match msg.guild(ctx).await {
        Some(i) => i,
        None => return Err(CommandError::from("Couldn't fetch details of this server.")),
    };

    let settings = match get_settings(ctx, &guild).await {
        Some(s) => s,
        None => {
            return Err(CommandError::from(
                "Couldn't fetch server details from the database.",
            ))
        }
    };

    match initial_checks(&settings) {
        Ok(_) => (),
        Err(e) => {
            msg.channel_id.say(&ctx.http, e).await?;
            return Ok(());
        }
    };

    if !(settings.total_players.unwrap_or(12) > settings.total_signups.unwrap_or(0)) {
        msg.channel_id
            .say(&ctx.http, "Maximum allowed players already signed up.")
            .await?;
        return Ok(());
    }

    let signups_channel = match get_signups_channel(&ctx, &guild, settings.signups_channel_id).await
    {
        Ok(c) => c,
        Err(e) => {
            msg.channel_id.say(&ctx.http, e).await?;
            return Ok(());
        }
    };

    if msg.channel_id != signups_channel.id {
        msg.channel_id
            .say(
                &ctx.http,
                "This command can only be used in the sign-ups channel.",
            )
            .await?;
        return Ok(());
    }

    // All first checks have been passed. We'll check if Player role exists now.
    let role = match get_role(ctx, guild.id, settings.player_role_id).await {
        Ok(r) => r,
        Err(_) => {
            msg.channel_id
                .say(&ctx.http, "Player role has not been set up.")
                .await?;
            return Ok(());
        }
    };

    let mut member = match get_member_and_add_role(ctx, msg, &role).await {
        Ok(m) => m,
        Err(e) => {
            msg.channel_id.say(&ctx.http, e).await?;
            return Ok(());
        }
    };

    remove_extra_roles(
        ctx,
        msg,
        &mut member,
        settings,
        &[Roles::Spectator, Roles::Replacement],
    )
    .await?;

    match &msg.react(&ctx.http, '✅').await {
        Ok(_) => (),
        Err(_) => {
            msg.channel_id.say(&ctx.http, "Added Player role!").await?;
        }
    };

    let data_read = ctx.data.read().await;
    let pool = data_read.get::<ConnectionPool>().unwrap();

    // Add one to total_signups.
    // Also add the player to the `players` array.
    sqlx::query!(
        "
        INSERT INTO config (guild_id, total_signups, players) VALUES ($1, 1, ARRAY[$2::bigint])
        ON CONFLICT (guild_id)
        DO UPDATE SET total_signups = coalesce(config.total_signups, 0) + 1,
        players = array_append(config.players, $2);
        ",
        guild.id.0 as i64,
        member.user.id.0 as i64
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Sign-out from the TvM or sign-up as a spectator.
///
/// **Usage:** `[p]out`
///
/// The command must be used in the sign-ups channel. It cannot be used
/// once the game has started. Please contact host directly if you'd like to
/// get the spectator role after the game has started.
#[command("out")]
#[aliases("spec")]
async fn sign_out(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = match msg.guild(ctx).await {
        Some(i) => i,
        None => return Err(CommandError::from("Couldn't fetch details of this server.")),
    };

    let settings = match get_settings(ctx, &guild).await {
        Some(s) => s,
        None => {
            return Err(CommandError::from(
                "Couldn't fetch server details from the database.",
            ))
        }
    };

    match initial_checks(&settings) {
        Ok(_) => (),
        Err(e) => {
            msg.channel_id.say(&ctx.http, e).await?;
            return Ok(());
        }
    };

    let signups_channel = match get_signups_channel(&ctx, &guild, settings.signups_channel_id).await
    {
        Ok(c) => c,
        Err(e) => {
            msg.channel_id.say(&ctx.http, e).await?;
            return Ok(());
        }
    };

    if msg.channel_id != signups_channel.id {
        msg.channel_id
            .say(
                &ctx.http,
                "This command can only be used in the sign-ups channel.",
            )
            .await?;
        return Ok(());
    }

    // All first checks have been passed. We'll check if Spectator role exists now.
    let role = match get_role(ctx, guild.id, settings.spec_role_id).await {
        Ok(r) => r,
        Err(_) => {
            msg.channel_id
                .say(&ctx.http, "Spectator role has not been set up.")
                .await?;
            return Ok(());
        }
    };

    let mut member = match get_member_and_add_role(ctx, msg, &role).await {
        Ok(m) => m,
        Err(e) => {
            msg.channel_id.say(&ctx.http, e).await?;
            return Ok(());
        }
    };

    remove_extra_roles(
        ctx,
        msg,
        &mut member,
        settings,
        &[Roles::Player, Roles::Replacement],
    )
    .await?;

    match &msg.react(&ctx.http, '✅').await {
        Ok(_) => (),
        Err(_) => {
            msg.channel_id
                .say(&ctx.http, "Added Spectator role!")
                .await?;
        }
    };

    Ok(())
}

/// Sign-up for the TvM as a replacement.
///
/// **Usage:** `[p]repl`
///
/// **Alias:** `replacement`
///
/// The command must be used in the sign-ups channel. It cannot be used
/// once the game has started. Please contact host directly if you'd like to
/// get the replacement role after the game starts.
#[command("repl")]
#[aliases("replacement")]
async fn sign_repl(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = match msg.guild(ctx).await {
        Some(i) => i,
        None => return Err(CommandError::from("Couldn't fetch details of this server.")),
    };

    let settings = match get_settings(ctx, &guild).await {
        Some(s) => s,
        None => {
            return Err(CommandError::from(
                "Couldn't fetch server details from the database.",
            ))
        }
    };

    match initial_checks(&settings) {
        Ok(_) => (),
        Err(e) => {
            msg.channel_id.say(&ctx.http, e).await?;
            return Ok(());
        }
    };

    let signups_channel = match get_signups_channel(&ctx, &guild, settings.signups_channel_id).await
    {
        Ok(c) => c,
        Err(e) => {
            msg.channel_id.say(&ctx.http, e).await?;
            return Ok(());
        }
    };

    if msg.channel_id != signups_channel.id {
        msg.channel_id
            .say(
                &ctx.http,
                "This command can only be used in the sign-ups channel.",
            )
            .await?;
        return Ok(());
    }

    // All first checks have been passed. We'll check if Spectator role exists now.
    let role = match get_role(ctx, guild.id, settings.repl_role_id).await {
        Ok(r) => r,
        Err(_) => {
            msg.channel_id
                .say(&ctx.http, "Replacement role has not been set up.")
                .await?;
            return Ok(());
        }
    };

    let mut member = match get_member_and_add_role(ctx, msg, &role).await {
        Ok(m) => m,
        Err(e) => {
            msg.channel_id.say(&ctx.http, e).await?;
            return Ok(());
        }
    };

    remove_extra_roles(
        ctx,
        msg,
        &mut member,
        settings,
        &[Roles::Player, Roles::Spectator],
    )
    .await?;

    match &msg.react(&ctx.http, '✅').await {
        Ok(_) => (),
        Err(_) => {
            msg.channel_id
                .say(&ctx.http, "Added Replacement role!")
                .await?;
        }
    };

    Ok(())
}

fn initial_checks(settings: &SignSettings) -> Result<(), &'static str> {
    if settings.cycle.number > 0 {
        return Err("You can't do that now. The game has started.");
    }

    if !settings.signups_on.unwrap_or(true) {
        return Err("Sign-ups are closed.");
    }

    Ok(())
}

async fn get_signups_channel(
    ctx: &Context,
    guild: &Guild,
    channel_id: Option<i64>,
) -> Result<GuildChannel, &'static str> {
    match channel_id {
        Some(i) => {
            if let Some(channel) = to_channel(ctx, guild.id, &i.to_string()).await {
                Ok(channel)
            } else {
                Err("Sign-ups channel has not been set up.")
            }
        }
        None => Err("Sign-ups channel has not been set up."),
    }
}

async fn get_settings(ctx: &Context, guild: &Guild) -> Option<SignSettings> {
    let data_read = ctx.data.read().await;
    let pool = data_read.get::<ConnectionPool>().unwrap();

    match sqlx::query_as_unchecked!(
        SignSettings,
        "
        SELECT
            cycle,
            signups_on,
            total_players,
            total_signups,
            signups_channel_id,
            player_role_id,
            spec_role_id,
            repl_role_id
        FROM config WHERE guild_id = $1;",
        guild.id.0 as i64
    )
    .fetch_one(pool)
    .await
    {
        Ok(s) => Some(s),
        Err(_) => None,
    }
}

async fn get_member_and_add_role<'a>(
    ctx: &Context,
    msg: &Message,
    role: &Role,
) -> Result<Member, Cow<'a, str>> {
    match msg.author.has_role(ctx, msg.guild_id.unwrap(), role).await {
        Ok(i) if i => {
            return Err(Cow::from(format!(
                "You already have the `{}` role.",
                role.name
            )))
        }
        _ => (),
    }

    let mut member = match msg.member(ctx).await {
        Ok(m) => m,
        Err(_) => return Err(Cow::from("I couldn't fetch details about you.")),
    };

    match member.add_role(ctx, role).await {
        Ok(_) => (),
        Err(_) => return Err(Cow::from(
            format!(
                "I either don't have the permissions to manage roles or the `{}` role is above my highest role.",
                role.name
            )
        ))
    };

    Ok(member)
}

async fn remove_extra_roles(
    ctx: &Context,
    msg: &Message,
    member: &mut Member,
    settings: SignSettings,
    extras: &[Roles],
) -> CommandResult {
    for extra in extras {
        match extra {
            Roles::Player => {
                remove_role(ctx, msg, member, settings.player_role_id, "Player").await?
            }
            Roles::Spectator => {
                remove_role(ctx, msg, member, settings.spec_role_id, "Spectator").await?
            }
            Roles::Replacement => {
                remove_role(ctx, msg, member, settings.repl_role_id, "Replacement").await?
            }
        };
    }

    Ok(())
}

async fn remove_role(
    ctx: &Context,
    msg: &Message,
    member: &mut Member,
    role_id: Option<i64>,
    role_type: &str,
) -> CommandResult {
    let role = match role_id {
        Some(i) => {
            if let Some(r) = to_role(ctx, member.guild_id, &i.to_string()).await {
                r
            } else {
                msg.channel_id
                    .say(
                        &ctx.http,
                        format!("{} role has not been set up.", role_type),
                    )
                    .await?;
                return Ok(());
            }
        }
        None => {
            msg.channel_id
                .say(
                    &ctx.http,
                    format!("{} role has not been set up.", role_type),
                )
                .await?;
            return Ok(());
        }
    };

    if member.user.has_role(ctx, member.guild_id, &role).await? {
        match member.remove_role(ctx, &role).await {
            Ok(_) => (),
            Err(_) => {
                msg.channel_id.say(
                    &ctx.http,
                    format!(
                        "I either don't have the permissions to manage roles or the `{}` role is above my highest role.",
                        role.name
                    )
                )
                .await?;
            }
        };

        if role_type == "Player" {
            // Decrease total_signups by one and remove player from players array.
            let data_read = ctx.data.read().await;
            let pool = data_read.get::<ConnectionPool>().unwrap();

            sqlx::query!(
                "
                INSERT INTO config (guild_id, total_signups) VALUES ($1, 0)
                ON CONFLICT (guild_id)
                DO UPDATE SET total_signups = coalesce(config.total_signups, 0) - 1,
                players = array_remove(config.players, $2);
                ",
                member.guild_id.0 as i64,
                member.user.id.0 as i64
            )
            .execute(pool)
            .await?;
        }
    }

    Ok(())
}

/// Lists all members with the Player role.
///
/// **Usage:** `[p]players [--all]`
///
/// By default, the bot only displays *alive* players. To show all players,
/// add "--all" after the command.
#[command("players")]
async fn all_players(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let guild = match msg.guild(ctx).await {
        Some(i) => i,
        None => return Err(CommandError::from("Couldn't fetch details of this server.")),
    };

    let data_read = ctx.data.read().await;
    let pool = data_read.get::<ConnectionPool>().unwrap();

    let res = match sqlx::query!(
        "SELECT player_role_id, players FROM config WHERE guild_id = $1;",
        guild.id.0 as i64
    )
    .fetch_one(pool)
    .await
    {
        Ok(i) => i,
        Err(_) => {
            return Err(CommandError::from(
                "Couldn't fetch details from the database.",
            ))
        }
    };

    let role = match get_role(ctx, guild.id, res.player_role_id).await {
        Ok(r) => r,
        Err(_) => {
            msg.channel_id
                .say(&ctx.http, "Player role has not been set up.")
                .await?;
            return Ok(());
        }
    };

    let players: Vec<_> = if !args.message().contains("--all") {
        guild
            .members
            .values()
            .filter_map(|m| {
                if m.roles.contains(&role.id) {
                    Some(m.mention().to_string())
                } else {
                    None
                }
            })
            .collect()
    } else {
        let player_ids = res.players.unwrap_or_default();

        guild
            .members
            .values()
            .filter_map(|m| {
                if player_ids.contains(&(m.user.id.0 as i64)) {
                    Some(m.mention().to_string())
                } else {
                    None
                }
            })
            .collect()
    };

    if players.is_empty() {
        msg.channel_id.say(&ctx.http, "No players!").await?;
        return Ok(());
    }

    let desc = players.join("\n");

    match msg
        .channel_id
        .send_message(&ctx.http, |m| {
            m.embed(|e| {
                e.description(&desc);
                e.title(format!("Total Players: {}", players.len()));
                e.colour(role.colour);

                e
            });

            m
        })
        .await
    {
        Ok(_) => (),
        Err(_) => {
            // Assuming we can't embed links.
            let options = ContentSafeOptions::new();
            msg.channel_id
                .say(
                    &ctx.http,
                    format!(
                        "**Total Players: {}\n\n{}",
                        players.len(),
                        content_safe(&ctx.cache, &desc, &options.display_as_member_from(guild))
                            .await
                    ),
                )
                .await?;
        }
    };

    Ok(())
}

/// Lists all members with the Replacement role.
///
/// **Usage:** `[p]replacements`
#[command("replacements")]
async fn all_replacements(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = match msg.guild(ctx).await {
        Some(i) => i,
        None => return Err(CommandError::from("Couldn't fetch details of this server.")),
    };

    let data_read = ctx.data.read().await;
    let pool = data_read.get::<ConnectionPool>().unwrap();

    let res = match sqlx::query!(
        "SELECT repl_role_id FROM config WHERE guild_id = $1;",
        guild.id.0 as i64
    )
    .fetch_one(pool)
    .await
    {
        Ok(i) => i,
        Err(_) => {
            return Err(CommandError::from(
                "Couldn't fetch details from the database.",
            ))
        }
    };

    let role = match get_role(ctx, guild.id, res.repl_role_id).await {
        Ok(r) => r,
        Err(_) => {
            msg.channel_id
                .say(&ctx.http, "Replacement role has not been set up.")
                .await?;
            return Ok(());
        }
    };

    let players: Vec<_> = guild
        .members
        .values()
        .filter_map(|m| {
            if m.roles.contains(&role.id) {
                Some(m.mention().to_string())
            } else {
                None
            }
        })
        .collect();

    if players.is_empty() {
        msg.channel_id.say(&ctx.http, "No replacements!").await?;
        return Ok(());
    }

    let desc = players.join("\n");

    match msg
        .channel_id
        .send_message(&ctx.http, |m| {
            m.embed(|e| {
                e.description(&desc);
                e.title(format!("Total Replacements: {}", players.len()));
                e.colour(role.colour);

                e
            });

            m
        })
        .await
    {
        Ok(_) => (),
        Err(_) => {
            // Assuming we can't embed links.
            let options = ContentSafeOptions::new();
            msg.channel_id
                .say(
                    &ctx.http,
                    format!(
                        "**Total Replacements: {}\n\n{}",
                        players.len(),
                        content_safe(&ctx.cache, &desc, &options.display_as_member_from(guild))
                            .await
                    ),
                )
                .await?;
        }
    };

    Ok(())
}

/// Displays the vote count.
///
/// **Usage:** `[p]votecount [channel] [--all]`
///
/// **Alias:** `vc`
///
/// Usually, the bot can automatically detect proper voting channels,
/// but it may fail to do so in some cases. Please specify the channel manually
/// if the bot is unable to detect the correct channel.
///
/// The bot only shows votes of *alive* players. If you want to get the votes of all players,
/// add "--all" at the end of command.
///
/// **Examples**
///
/// Command: `[p]vc`
/// Result: The bot tries to find voting channel and displays votes of *alive* players if it
/// can find the channel.
///
/// Command: `[p]vc #day-1-voting`
/// Result: The bot displays votes of *alive* players from #day-1-voting channel.
///
/// Command: `[p]vc --all`
/// Result: The bot tries to find voting channel and displays votes of *all* players if it
/// can find the channel.
///
/// Command: `[p]vc #day-1-voting --all`
/// Result: The bot displays votes of *all* players from #day-1-voting channel.
#[command("votecount")]
#[aliases("vc")]
async fn vote_count(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let guild = match msg.guild(ctx).await {
        Some(i) => i,
        None => return Err(CommandError::from("Couldn't fetch details of this server.")),
    };

    let data_read = ctx.data.read().await;
    let pool = data_read.get::<ConnectionPool>().unwrap();

    let data: VoteData = match sqlx::query_as_unchecked!(
        VoteData,
        "SELECT player_role_id, players, cycle FROM config WHERE guild_id = $1",
        guild.id.0 as i64
    )
    .fetch_one(pool)
    .await
    {
        Ok(d) => d,
        Err(_) => {
            return Err(CommandError::from(
                "Couldn't fetch details from the database.",
            ))
        }
    };

    let cycle = match data.cycle {
        Some(c) => c.0,
        None => Cycle {
            number: 0,
            day: None,
            night: None,
            votes: None,
        },
    };

    // Time for argument parsing
    let all = args.message().contains("--all");

    // Check if user passed a channel.
    let channel = match get_channel(
        ctx,
        guild.id,
        Some(&args.message().replace("--all", "").to_string()),
    )
    .await
    {
        Ok(c) => c,
        Err(_) => {
            // See if `cycle` has voting channel.
            match get_channel_from_id(ctx, guild.id, cycle.votes).await {
                Ok(c) => c,
                Err(_) => {
                    msg.channel_id.say(
                        &ctx.http,
                        "
                        The game doesn't appear to have begun. \
                        If it has, please ask a host to use the `started` command.\
                        \n\nMeanwhile, you can use `votecount` command by passing the voting channel \
                        after the command, like `votecount #channel-name`.
                        "
                    )
                    .await?;

                    return Ok(());
                }
            }
        }
    };

    // Let's check if `Player` role exists. We need it to filter messages.
    let role = match get_role(ctx, guild.id, data.player_role_id).await {
        Ok(r) => r,
        Err(_) => {
            msg.channel_id
                .say(&ctx.http, "Player role couldn't be found.")
                .await?;
            return Ok(());
        }
    };

    let players: HashSet<_> = if !all {
        guild
            .members
            .into_iter()
            .filter_map(|(_, m)| {
                if m.roles.contains(&role.id) {
                    Some(m.user)
                } else {
                    None
                }
            })
            .collect()
    } else {
        let player_ids = data.players.unwrap_or_default();
        guild
            .members
            .into_iter()
            .filter_map(|(_, m)| {
                if player_ids.contains(&(m.user.id.0 as i64)) {
                    Some(m.user)
                } else {
                    None
                }
            })
            .collect()
    };

    let mut user_votes = HashMap::new();
    let mut messages = channel.id.messages_iter(&ctx).boxed();
    while let Some(message) = messages.next().await {
        if let Ok(message) = message {
            if !players.contains(&message.author) || user_votes.contains_key(&message.author) {
                continue;
            }
            let vote_res = get_vote_from_message(clean_user_mentions(&message));
            if let Some(vote) = vote_res {
                user_votes.insert(message.author, Some(vote));
            }
        }
    }

    // Adds non-voters to `user_votes`.
    get_non_voters(players, &mut user_votes);

    // Now that we have a `HashMap` of `user -> vote`, we'll create a IndexMap
    // of `vote -> Vec<user>`. We use an `IndexMap` because ordering matters now.
    // Instead of using 4 separate vectors with users, we used a `user -> vote` `HashMap`
    // because keys in hash maps are unique. It makes sure a user's vote is only counted once.
    let mut votes: IndexMap<_, Vec<_>> = IndexMap::new();
    for (user, vote) in user_votes {
        if votes.contains_key(&vote) {
            votes.get_mut(&vote).unwrap().push(user);
        } else {
            votes.insert(vote, vec![user]);
        }
    }

    // Sort the mapping to get the most votes at the top.
    votes.sort_by(|_, v1, _, v2| v1.len().cmp(&v2.len()).reverse());

    // Remove and add "VTNL" and "No vote" for ordering
    if let Some(v) = votes.shift_remove(&Some(Vote::Vtnl)) {
        votes.insert(Some(Vote::Vtnl), v);
    };
    if let Some(v) = votes.shift_remove(&None) {
        votes.insert(None, v);
    };

    // String to display formatted votes.
    let mut votes_str = String::new();
    for (idx, (vote, voters)) in votes.iter().enumerate() {
        let voters: Vec<_> = voters
            .iter()
            .map(|m| format!("{}#{}", m.name, m.discriminator))
            .collect();

        match vote {
            Some(v) => match v {
                Vote::Vtnl => write!(
                    votes_str,
                    "\n\n**VTNL** - {} ({})",
                    voters.len(),
                    voters.join(", ")
                )?,
                Vote::Vtl(s) => write!(
                    votes_str,
                    "\n{}. **{}** - {} ({})",
                    idx + 1,
                    s,
                    voters.len(),
                    voters.join(", ")
                )?,
                _ => (),
            },
            None => write!(
                votes_str,
                "\n\n**Not voting** - {} ({})",
                voters.len(),
                voters.join(", ")
            )?,
        };
    }

    let desc = format!(
        "__Counting from {} channel.__\n\n{}",
        channel.mention(),
        votes_str.trim()
    );

    let rep = msg
        .channel_id
        .send_message(&ctx.http, |m| {
            m.embed(|e| {
                e.colour(0x00CDFF);
                e.title("Vote Count");
                e.description(&desc);

                e
            });

            m
        })
        .await;

    if rep.is_err() {
        // Assuming the error was related to permission to send an embed.
        msg.channel_id
            .say(&ctx.http, format!("**Vote Count**\n\n{}", &desc))
            .await?;
    }

    Ok(())
}

fn get_vote_from_message(content: String) -> Option<Vote> {
    let vote_re: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"^[\*_~|]*[Vv][Tt][Ll][\*_~|]*[\s\*_~|]+([^\*_~|]+)").unwrap());
    let un_vote_re: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"^[\*_~|]*[Uu][Nn]-?[Vv][Tt][Ll][\*_~|]*[\s\*_~|]+([^\*_~|]+)?").unwrap()
    });
    let vtnl_re: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"^[\*_~|]*[Vv][Tt][Nn][Ll][\*_~|]*").unwrap());

    if let Some(c) = vote_re.captures(content.as_str()) {
        return Some(Vote::Vtl(capitalize(c.get(1).map_or("", |m| m.as_str()))));
    };

    if let Some(c) = un_vote_re.captures(content.as_str()) {
        return Some(Vote::UnVtl(capitalize(c.get(1).map_or("", |m| m.as_str()))));
    };

    if vtnl_re.is_match(content.as_str()) {
        Some(Vote::Vtnl)
    } else {
        None
    }
}

fn get_non_voters(players: HashSet<User>, votes: &mut HashMap<User, Option<Vote>>) {
    for player in players {
        votes.entry(player).or_insert(None);
    }
}

/// Time elapsed since the first message in the current phase channel.
///
/// **Usage:** `[p]timesince`
///
/// **Alias:** `ts`
#[command("timesince")]
#[aliases("ts")]
async fn time_since(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = match msg.guild(ctx).await {
        Some(i) => i,
        None => return Err(CommandError::from("Couldn't fetch details of this server.")),
    };

    let data_read = ctx.data.read().await;
    let pool = data_read.get::<ConnectionPool>().unwrap();

    let data = match sqlx::query_as_unchecked!(
        Data,
        "SELECT player_role_id, cycle FROM config WHERE guild_id = $1",
        guild.id.0 as i64
    )
    .fetch_one(pool)
    .await
    {
        Ok(d) => d,
        Err(_) => {
            return Err(CommandError::from(
                "Couldn't fetch details from the database.",
            ))
        }
    };

    let cycle = match data.cycle {
        Some(c) => c.0,
        None => {
            msg.channel_id
                .say(&ctx.http, "Game doesn't appear to have started.")
                .await?;
            return Ok(());
        }
    };

    let role = match get_role(ctx, guild.id, data.player_role_id).await {
        Ok(r) => r,
        Err(_) => {
            msg.channel_id
                .say(&ctx.http, "Player role couldn't be found.")
                .await?;
            return Ok(());
        }
    };

    let day = match get_channel_from_id(ctx, guild.id, cycle.day).await {
        Ok(c) => c,
        Err(_) => {
            msg.channel_id
                .say(&ctx.http, "Day channel couldn't be fetched.")
                .await?;
            return Ok(());
        }
    };

    let (channel, phase) = if is_day(&day, &role) {
        (day, format!("Day {}", cycle.number))
    } else {
        let night = match get_channel_from_id(ctx, guild.id, cycle.night).await {
            Ok(c) => c,
            Err(_) => {
                msg.channel_id
                    .say(&ctx.http, "Night channel couldn't be fetched.")
                    .await?;
                return Ok(());
            }
        };

        (night, format!("Night {}", cycle.number))
    };

    // A workaround to get the first message in the channel by passing channel's id in `after`.
    let message = match channel
        .messages(&ctx.http, |ret| ret.limit(1).after(channel.id.0))
        .await
    {
        Ok(m) => {
            if !m.is_empty() {
                m[0].clone()
            } else {
                msg.channel_id
                    .say(
                        &ctx.http,
                        format!("The {} channel seems empty.", channel.mention()),
                    )
                    .await?;
                return Ok(());
            }
        }
        Err(_) => {
            return Err(CommandError::from(
                "I couldn't fetch the first message in the channel.",
            ))
        }
    };

    let mut duration = format_duration(Utc::now().signed_duration_since(message.timestamp));
    // let duration_st;
    if duration.trim().is_empty() {
        duration = format!("{} began a few seconds ago.", phase);
    } else {
        duration = format!("{} began about {} ago.", phase, duration);
    }

    msg.channel_id.say(&ctx.http, duration).await?;

    Ok(())
}

fn is_day(day_channel: &GuildChannel, player_role: &Role) -> bool {
    for overwrites in &day_channel.permission_overwrites {
        match overwrites.kind {
            PermissionOverwriteType::Role(r) => {
                if r == player_role.id {
                    if overwrites.allow.send_messages() {
                        return true;
                    }
                } else if r.0 == day_channel.guild_id.0 && overwrites.allow.send_messages() {
                    return true;
                }
            }
            _ => continue,
        }
    }

    false
}

/// Returns string representing the `Duration` in a humanized form.
/// It is designed to return duration in terms of days, hours and minutes only.
///
/// In cases when the time duration is smaller than a minute, an empty string is returned.
///
/// Source: https://github.com/Cog-Creators/Red-DiscordBot/blob/V3/develop/redbot/core/utils/chat_formatting.py#L419
fn format_duration(duration: Duration) -> String {
    let mut total_seconds = duration.num_seconds();

    let periods = [
        ("day", "days", 60 * 60 * 24),
        ("hour", "hours", 60 * 60),
        ("minute", "minutes", 60),
    ];

    let mut strings: Vec<String> = Vec::new();
    for (name, plural_name, seconds) in periods.iter() {
        if total_seconds >= *seconds {
            let value = total_seconds / seconds;
            total_seconds %= seconds;
            if value == 0 {
                continue;
            }
            let unit = if value > 1 { plural_name } else { name };
            strings.push(format!("{} {}", value, unit));
        }
    }

    strings.join(", ")
}

/// Submits your action for the night.
///
/// **Usage:** `[p]nightaction <action_message>`
///
/// **Alias:** `na`
///
/// If the host has disabled night action changes, then you will not
/// be able to change your night action once submitted.
///
/// If user has allowed night action changes, using the same command again
/// will update your night action.
#[command("nightaction")]
#[aliases("na")]
#[min_args(1)]
async fn night_action(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    // It won't be empty because of the `min_args` check.
    let action = args.message();

    let guild = match msg.guild(ctx).await {
        Some(i) => i,
        None => return Err(CommandError::from("Couldn't fetch details of this server.")),
    };

    // TODO: Add a day check.

    match get_channel_from_id(ctx, guild.id, Some(msg.channel_id.0 as i64)).await {
        Ok(c) => {
            let private = c.permission_overwrites.iter().any(|p| match p.kind {
                PermissionOverwriteType::Member(m) => {
                    if m == msg.author.id {
                        p.allow.send_messages()
                    } else {
                        false
                    }
                }
                _ => false,
            });

            if !private {
                msg.channel_id.say(
                    &ctx.http,
                    "This doesn't look like your private channel. This command can only be used in your private channel."
                ).await?;
                return Ok(());
            }
        }
        Err(_) => {
            msg.channel_id
                .say(&ctx.http, "Unable to get details of this channel.")
                .await?;
            return Ok(());
        }
    }

    let mut title = format!("{}'s Night Action", msg.author.name);

    let data_read = ctx.data.read().await;
    let pool = data_read.get::<ConnectionPool>().unwrap();

    let data = match sqlx::query!(
        "SELECT na_submitted, can_change_na FROM config WHERE guild_id = $1",
        guild.id.0 as i64
    )
    .fetch_one(pool)
    .await
    {
        Ok(d) => d,
        Err(_) => {
            return Err(CommandError::from(
                "Couldn't fetch details from the database.",
            ))
        }
    };

    let na_submitted: Vec<i64> = match data.na_submitted {
        Some(n) => n,
        None => Vec::new(),
    };

    let can_change_na = data.can_change_na.unwrap_or(true);

    if na_submitted.contains(&(msg.author.id.0 as i64)) {
        if !can_change_na {
            msg.channel_id
                .say(&ctx.http, "You've already submitted a night action.")
                .await?;
            return Ok(());
        } else {
            title.push_str(" (Updated)");
        }
    }

    let na_channel = match get_na_channel(ctx, &guild, &pool).await {
        Ok(c) => c,
        Err(e) => {
            return Err(CommandError::from(e));
        }
    };

    na_channel
        .say(&ctx.http, format!("**{}**\n{}", title, action))
        .await?;

    if !na_submitted.contains(&(msg.author.id.0 as i64)) {
        sqlx::query!(
            "
            INSERT INTO config(
                guild_id, na_submitted
            ) VALUES (
                $1, array[$2]::bigint[]
            ) ON CONFLICT (guild_id)
            DO UPDATE SET na_submitted = array_append(config.na_submitted, $2)
            WHERE config.na_submitted IS NULL
            OR not(config.na_submitted @> array[$2]::bigint[]);
            ",
            guild.id.0 as i64,
            msg.author.id.0 as i64
        )
        .execute(pool)
        .await?;
    }

    msg.channel_id
        .say(&ctx.http, "Submitted night action!")
        .await?;

    Ok(())
}

/// Parses supplied CommonMark Markdown text and attaches formatted JPEG and PDF.
///
/// **Usage:** `[p]format <message>`
///
/// This command takes CommonMark-flavoured Markdown and attaches a nicely formatted
/// JPEG image and a PDF. This allows users to use richer Markdown than supported by Discord.
///
/// Some extra features supported:
///    - Nested quotes
///    - Tables
///    - Lists
///    - Headings (6 levels)
///    - Hyperlinks (they only work in PDFs)
///    - Horizontal rules
///
/// Syntax to use these features:
///
/// ```md
/// # Level 1 Heading
/// ## Level 2 Heading
/// ###### Level 6 Heading (can't go any deeper)
///
/// > This is a single quote.
/// >> This quote is inside the first quote.
/// >>> This is inside the second quote. Three nest layers!
///
/// > Back to single quote. The blank line above is required.
///
/// And finally, no quotes. Again, the blank line is required.
///
/// Let's add a horizontal rule below.
/// ***
/// That was a horizontal rule.
///
/// Now, we'll add a [hyperlink](https://www.google.com/). This is only clickable
/// in the PDF.
/// ```
///
/// For a complete guide, see [this page](https://ariusx7.github.io/tvm-assistant/formatting)
#[command("format")]
#[min_args(1)]
async fn format_text(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let text = args.message();

    // The conversion process is very expensive. Before converting, let's make sure
    // we can attach images and files.
    if let Some(c) = msg.channel(&ctx.cache).await {
        if let Some(channel) = c.guild() {
            let user_id = &ctx.cache.current_user_id().await;
            if let Ok(perms) = channel.permissions_for_user(&ctx.cache, user_id).await {
                if !perms.attach_files() {
                    msg.channel_id
                        .say(&ctx.http, "I cannot attach files in this channel.")
                        .await?;
                    return Ok(());
                }
            }
        }
    };

    // We'll initiate typing so that the user doesn't think bot went offline or broke.
    // If there is any error with this, we'll handle it silently.
    let _ = msg.channel_id.broadcast_typing(&ctx.http).await;

    let files = match markdown_to_files(text).await {
        (Some(p), Some(i)) => vec![i, p],
        (Some(p), None) => vec![p],
        (None, Some(i)) => vec![i],
        (None, None) => return Ok(()),
    };

    msg.channel_id
        .send_files(&ctx.http, files, |m| {
            m.content(format!("{} sent the following:", msg.author.mention()));

            m
        })
        .await?;

    clean_files();

    Ok(())
}

/// Deletes files created by `markdown_to_files` function.
fn clean_files() {
    // `foo.html`
    if fs::remove_file("foo.html").is_err() {
        error!("Error removing `foo.html`.");
    };

    // `out.pdf`
    if fs::remove_file("out.pdf").is_err() {
        error!("Error removing `out.pdf`.");
    };

    // `out.jpeg`
    if fs::remove_file("out.jpeg").is_err() {
        error!("Error removing `out.jpeg`.");
    };
}

/// Searches for a page on Town of Salem wikia (fandom).
///
/// **Usage:** `[p]tos <name>`
///
/// The bot displays at most 5 results that match the entered `name`. A page is
/// considered to be a result if it's title contains `name`. If only one result is
/// found, [botname] just posts the link to the wikia page. If more than one results
/// are found, the bot sends an embed with hyperlinks to all of the results.
///
/// **Examples**
///
/// Command: `[p]tos doctor`
/// Result: https://town-of-salem.fandom.com/wiki/Doctor
///
/// Command: `[p]tos invest`
/// Result: Links to five pages with "invest" in their titles.
#[command("tos")]
async fn tos_wiki(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let input = if !args.message().is_empty() {
        args.message()
    } else {
        msg.channel_id
            .say(
                &ctx.http,
                "https://town-of-salem.fandom.com/wiki/Town_of_Salem_Wiki:Main_Page",
            )
            .await?;
        return Ok(());
    };

    // Get request client from data.
    let data = ctx.data.read().await;
    let client = data.get::<RequestClient>().unwrap();

    let results = tos::search(client, input).await?;

    let mut desc = String::new();
    if results.is_empty() {
        msg.channel_id.say(&ctx.http, "No results found.").await?;
        return Ok(());
    } else {
        if results.len() == 1 {
            msg.channel_id.say(&ctx.http, &results[0].url).await?;
            return Ok(());
        }

        for (idx, item) in results.iter().enumerate() {
            let _ = write!(desc, "\n{}. [{}]({})", idx + 1, item.title, item.url);
        }
    }

    msg.channel_id
        .send_message(&ctx.http, |m| {
            m.embed(|e| {
                e.description(desc);
                e.colour(EMBED_COLOUR);
                e.title("Results");

                e
            });

            m
        })
        .await?;

    Ok(())
}

/// Sends jump url for the first message of a channel.
///
/// **Usage:** `[p]top [channel]`
///
/// You can supply a channel to get it's first message. If you don't supply a channel,
/// the bot will send link for the first message of the channel where the command is used.
#[command("top")]
async fn top_cmd(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    // Get the current channel
    let channel_id = match get_channel(
        &ctx,
        msg.guild_id.unwrap_or(GuildId(0)),
        Some(&args.message().to_string()),
    )
    .await
    {
        Ok(c) => c.id,
        Err(_) => msg.channel_id,
    };

    // A workaround to get the first message in the channel by passing channel's id in `after`.
    let first_message = match channel_id
        .messages(&ctx.http, |ret| ret.limit(1).after(channel_id.0))
        .await
    {
        Ok(m) => {
            if !m.is_empty() {
                m[0].clone()
            } else {
                msg.channel_id
                    .say(
                        &ctx.http,
                        format!("The {} channel seems to be empty.", channel_id.mention()),
                    )
                    .await?;
                return Ok(());
            }
        }
        Err(_) => {
            return Err(CommandError::from(
                "I couldn't fetch the first message in the channel.",
            ))
        }
    };

    let url = get_jump_url_with_guild(&first_message, &msg.guild_id.unwrap());

    let sent = msg
        .channel_id
        .send_message(&ctx.http, |m| {
            m.embed(|e| {
                e.colour(EMBED_COLOUR);
                e.description(format!("[Click here to jump to the top.]({})", url));

                e
            });

            m
        })
        .await;

    if sent.is_err() {
        // Send the url directly instead of an error message.
        msg.channel_id.say(&ctx.http, url).await?;
    }

    Ok(())
}

/// Shows a user's voting history.
///
/// **Usage:** `[p]votehistory [channel] <user>`
///
/// **Alias:** `vh`
///
/// If the host has used `cycle` commands to create cycle channels, the bot will
/// know which is the latest voting channel. The results will be displayed by considering
/// the votes in that channel. If the bot is unable to detect a voting channel, you'll have
/// to pass the channel before the user.
///
/// **Examples**
///
/// *Assuming host used `cycle` command and the latest voting channel is `day-5-voting`*
///
/// Command: `[p]vh Arius`
/// Result: The bot will show `Arius`' vote history from `#day-5-voting` channel.
///
/// *If the host didn't use `cycle` command or if you wish to get votes from a specific channel*
///
/// Command: `[p]vh #day-5-voting Arius`
/// Result: The bot will show `Arius`' vote history from `#day-5-voting` channel.
#[command("votehistory")]
#[aliases("vh")]
#[min_args(1)]
async fn vote_history(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild = match msg.guild(ctx).await {
        Some(i) => i,
        None => return Err(CommandError::from("Couldn't fetch details of this server.")),
    };

    // Get channel if passed.
    let (passed_channel, user_arg) = match args.single::<String>() {
        Ok(arg) => match get_channel(&ctx, guild.id, Some(&arg)).await {
            Ok(c) => (Some(c), args.remains()),
            Err(_) => (None, Some(args.message())),
        },
        Err(_) => return Err(CommandError::from("There was an unexpected error.")),
    };

    let user = match user_arg {
        Some(a) => match get_member(&ctx, guild.id, Some(&a.to_string())).await {
            Ok(m) => m,
            Err(_) => {
                msg.channel_id
                    .say(&ctx.http, format!("No user found from `{}`.", a))
                    .await?;
                return Ok(());
            }
        },
        None => {
            msg.channel_id
                .say(&ctx.http, "A user must be passed for this command to work.")
                .await?;
            return Ok(());
        }
    };

    // Query database for player_role_id and cycle data.
    let data_read = ctx.data.read().await;
    let pool = data_read.get::<ConnectionPool>().unwrap();

    let cycle: Cycle = match sqlx::query_as_unchecked!(
        CycleContainer,
        "SELECT cycle FROM config WHERE guild_id = $1",
        guild.id.0 as i64
    )
    .fetch_one(pool)
    .await
    {
        Ok(cf) => {
            if let Some(c) = cf.cycle {
                c.0
            } else {
                Cycle {
                    number: 0,
                    day: None,
                    night: None,
                    votes: None,
                }
            }
        }
        Err(_) => {
            return Err(CommandError::from(
                "Couldn't fetch details from the database.",
            ))
        }
    };

    // Check if user passed a channel.
    let channel = match passed_channel {
        Some(c) => c,
        None => {
            // See if `cycle` has voting channel.
            match get_channel_from_id(ctx, guild.id, cycle.votes).await {
                Ok(c) => c,
                Err(_) => {
                    msg.channel_id.say(
                        &ctx.http,
                        "
                        The game doesn't appear to have begun. \
                        If it has, please ask a host to use the `started` command.\
                        \n\nMeanwhile, you can use `votehistory` command by passing the voting channel \
                        after the command, like `votehistory #channel-name <user>`.
                        "
                    )
                    .await?;

                    return Ok(());
                }
            }
        }
    };

    // We'll get the messages now and process them.
    let mut messages = match channel.messages(&ctx.http, |ret| ret.limit(100)).await {
        Ok(v) => v,
        Err(_) => return Err(CommandError::from("I was unable to get messages.")),
    };
    messages.reverse();

    let mut votes_str = String::new();
    let mut count = 0;
    for message in &messages {
        if message.author.id != user.user.id {
            continue;
        }
        if let Some(vote) = get_vote_from_message(clean_user_mentions(message)) {
            count += 1;
            let _ = match vote {
                Vote::Vtl(u) => write!(votes_str, "\n{}. **VTL {}**", count, u),
                Vote::UnVtl(u) => write!(votes_str, "\n{}. **UnVTL {}**", count, u),
                Vote::Vtnl => write!(votes_str, "\n{}. **VTNL**", count),
            };

            let _ = write!(
                votes_str,
                " (on {} {})",
                format_day(message.timestamp.day()),
                message.timestamp.format("%B at %-I:%M %P")
            );
        }
    }

    if votes_str.is_empty() {
        let _ = write!(votes_str, "No votes.");
    }

    let sent = msg
        .channel_id
        .send_message(&ctx.http, |m| {
            m.embed(|e| {
                e.colour(EMBED_COLOUR);
                e.description(format!(
                    "Considering votes in {} channel.\n{}",
                    channel.mention(),
                    votes_str
                ));
                e.author(|a| {
                    a.name(format!("{}'s Voting History", user.user.name));
                    a.icon_url(user.user.face());

                    a
                });
                e.footer(|f| {
                    f.text("All times are in UTC.");

                    f
                });

                e
            });

            m
        })
        .await;

    if sent.is_err() {
        msg.channel_id
            .say(
                &ctx.http,
                "I need embed links permission to display vote count.",
            )
            .await?;
    }

    Ok(())
}

fn format_day(day: u32) -> String {
    if day == 1 || day == 21 || day == 31 {
        format!("{}st", day)
    } else if day == 2 || day == 22 {
        format!("{}nd", day)
    } else if day == 3 || day == 23 {
        format!("{}rd", day)
    } else {
        format!("{}th", day)
    }
}

/// Pings the Players role to notify them of a message.
///
/// **Usage:** `[p]notify <msg>`
///
/// The command has a cooldown to discourage spam mentions. The cooldown defaults to
/// once per 6 hours in a particular server. A server's cooldown can be changed by a host.
#[command]
#[min_args(1)]
async fn notify(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let guild = match msg.guild(ctx).await {
        Some(i) => i,
        None => return Err(CommandError::from("Couldn't fetch details of this server.")),
    };

    let data_read = ctx.data.read().await;
    let pool = data_read.get::<ConnectionPool>().unwrap();

    let lu_res_opt = sqlx::query!(
        "SELECT last_used FROM cooldown WHERE guild_id = $1 AND cmd = $2",
        guild.id.0 as i64,
        "notify",
    )
    .fetch_optional(pool)
    .await?;

    let now = Utc::now();

    if let Some(lu_res) = lu_res_opt {
        if let Some(last_used) = lu_res.last_used {
            let cd_res = sqlx::query!(
                "SELECT notify_cooldown FROM config WHERE guild_id = $1",
                guild.id.0 as i64,
            )
            .fetch_one(pool)
            .await?;

            let diff = last_used + Duration::hours(cd_res.notify_cooldown.into()) - now;
            if diff.num_seconds() > 0 {
                let duration_str = format_duration(diff);
                let formatted_dur = if duration_str.trim().is_empty() {
                    Cow::from("a few seconds")
                } else {
                    Cow::from(duration_str)
                };

                msg.channel_id
                    .say(
                        &ctx.http,
                        format!(
                            "This command is on a cooldown. Try again in {}.",
                            formatted_dur
                        ),
                    )
                    .await?;
                return Ok(());
            }
        }
    }

    let res = sqlx::query!(
        "SELECT player_role_id FROM config WHERE guild_id = $1",
        guild.id.0 as i64
    )
    .fetch_one(pool)
    .await?;

    let role = if let Ok(r) = get_role(ctx, guild.id, res.player_role_id).await {
        r
    } else {
        msg.channel_id
            .say(&ctx.http, "Player role has not been set up.")
            .await?;

        return Ok(());
    };

    msg.channel_id
        .say(
            &ctx.http,
            format!(
                "{role}, {player} chose to notify everyone:\n\n>>> {msg}",
                role = role.mention(),
                player = msg.author.mention(),
                msg = args.rest(),
            ),
        )
        .await?;

    sqlx::query!(
        r#"
        INSERT INTO cooldown VALUES (
            $1,
            $2,
            $3
        ) ON CONFLICT (guild_id, cmd)
        DO UPDATE SET last_used = $3;
        "#,
        guild.id.0 as i64,
        "notify",
        now,
    )
    .execute(pool)
    .await?;

    Ok(())
}

#[group("General")]
#[only_in("guilds")]
#[commands(
    sign_in,
    sign_out,
    sign_repl,
    all_players,
    all_replacements,
    vote_count,
    time_since,
    night_action,
    format_text,
    tos_wiki,
    top_cmd,
    vote_history,
    notify
)]
#[description("General commands for users.")]
struct UserCommands;
