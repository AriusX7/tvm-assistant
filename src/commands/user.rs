// This module contains commands and related functiosn for general
// users, like players, spectators and replacements.

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
        tos::get_items,
    },
    ConnectionPool, RequestClient,
};
use chrono::{offset::Utc, Datelike, Duration};
use indexmap::IndexMap;
use log::error;
use regex::Regex;
use serenity::{
    framework::standard::{
        macros::{command, group},
        Args, CommandError, CommandResult,
    },
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
use std::{borrow::Cow, collections::HashMap, fmt::Write, fs};

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

#[derive(Debug)]
enum Vote {
    VTL(String),
    UnVTL(String),
    VTNL,
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
    sqlx::query!(
        "
        INSERT INTO config (guild_id, total_signups) VALUES ($1, 1)
        ON CONFLICT (guild_id)
        DO UPDATE SET total_signups = coalesce(config.total_signups, 0) + 1;
        ",
        guild.id.0 as i64
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
        Some(m) => m,
        None => return Err(Cow::from("I couldn't fetch details about you.")),
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
            // Decrease total_signups by one.
            let data_read = ctx.data.read().await;
            let pool = data_read.get::<ConnectionPool>().unwrap();

            sqlx::query!(
                "
                INSERT INTO config (guild_id, total_signups) VALUES ($1, 0)
                ON CONFLICT (guild_id)
                DO UPDATE SET total_signups = coalesce(config.total_signups, 0) - 1;
                ",
                member.guild_id.0 as i64
            )
            .execute(pool)
            .await?;
        }
    }

    Ok(())
}

/// Lists all members with the Player role.
///
/// **Usage:** `[p]players`
#[command("players")]
async fn all_players(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = match msg.guild(ctx).await {
        Some(i) => i,
        None => return Err(CommandError::from("Couldn't fetch details of this server.")),
    };

    let data_read = ctx.data.read().await;
    let pool = data_read.get::<ConnectionPool>().unwrap();

    let res = match sqlx::query!(
        "SELECT player_role_id FROM config WHERE guild_id = $1;",
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

    let players: Vec<String> = guild
        .members
        .values()
        .filter_map(|m| {
            if m.roles.contains(&role.id) {
                Some(m.mention())
            } else {
                None
            }
        })
        .collect();

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

    let players: Vec<String> = guild
        .members
        .values()
        .filter_map(|m| {
            if m.roles.contains(&role.id) {
                Some(m.mention())
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
/// **Usage:** `[p]votecount [channel]`
///
/// **Alias:** `vc`
///
/// Usually, the bot can automatically detect proper voting channels,
/// but it may fail to do so in some cases. Please specify the channel manually
/// if the bot is unable to detect the correct channel.
#[command("votecount")]
#[aliases("vc")]
async fn vote_count(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
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
        None => Cycle {
            number: 0,
            day: None,
            night: None,
            votes: None,
        },
    };

    // Check if user passed a channel.
    // args.message()
    let channel = match get_channel(ctx, guild.id, Some(&args.message().to_string())).await {
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
                        If it has, please use a host to use the `started` command.\
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

    let players: Vec<&User> = guild
        .members
        .values()
        .filter_map(|m| {
            if m.roles.contains(&role.id) {
                Some(&m.user)
            } else {
                None
            }
        })
        .collect();

    let mut messages = match channel.messages(&ctx.http, |ret| ret.limit(100)).await {
        Ok(m) => m,
        Err(_) => return Err(CommandError::from("I was unable to get messages.")),
    };
    // Messages are ordered from new to oldest. We need to reverse that.
    messages.reverse();

    let mut user_votes = HashMap::new();
    for message in &messages {
        if !players.contains(&&message.author) {
            continue;
        }
        let vote_res = get_vote_from_message(clean_user_mentions(&message));
        if let Some(vote) = vote_res {
            match vote {
                Vote::VTL(u) => {
                    user_votes.insert(&message.author, u);
                }
                Vote::UnVTL(u) => {
                    if user_votes.get(&message.author).unwrap_or(&"".to_string()) == &u
                        && !u.is_empty()
                    {
                        user_votes.remove_entry(&message.author);
                    }
                }
                Vote::VTNL => {
                    user_votes.insert(&message.author, "VTNL".to_string());
                }
            }
        }
    }

    // Adds non-voters to `user_votes`.
    get_non_voters(players, &mut user_votes);

    // Now that we have a `HashMap` of `user -> vote`, we'll create a IndexMap
    // of `vote -> Vec<user>`. We use an `IndexMap` because ordering matters now.
    // Instead of using 4 separate vectors with users, we used a `user -> vote` `HashMap`
    // because keys in hash maps are unique. It makes sure a user's vote is only counted once.

    let mut votes: IndexMap<String, Vec<&User>> = IndexMap::new();
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
    if let Some(v) = votes.shift_remove("VTNL") {
        votes.insert(String::from("VTNL"), v);
    };
    if let Some(v) = votes.shift_remove("No vote") {
        votes.insert(String::from("No vote"), v);
    };

    // String to display formatted votes.
    let mut votes_str = String::new();
    for (idx, vote) in votes.iter().enumerate() {
        let voters: Vec<_> = vote
            .1
            .iter()
            .map(|m| format!("{}#{}", m.name, m.discriminator))
            .collect();
        let vote = vote.0.as_str();

        match vote {
            "VTNL" => write!(
                votes_str,
                "\n\n**VTNL** - {} ({})",
                voters.len(),
                voters.join(", ")
            )?,
            "No vote" => write!(
                votes_str,
                "\n\n**Not voting** - {} ({})",
                voters.len(),
                voters.join(", ")
            )?,
            _ => write!(
                votes_str,
                "\n{}. **{}** - {} ({})",
                idx + 1,
                vote,
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
    lazy_static! {
        static ref VOTE_RE: Regex =
            Regex::new(r"^[\*_~|]*[Vv][Tt][Ll][\*_~|]*[\s\*_~|]+([^\*_~|]+)").unwrap();
        static ref UN_VOTE_RE: Regex =
            Regex::new(r"^[\*_~|]*[Uu][Nn]-?[Vv][Tt][Ll][\*_~|]*[\s\*_~|]+([^\*_~|]+)?").unwrap();
        static ref VTNL_RE: Regex = Regex::new(r"^[\*_~|]*[Vv][Tt][Nn][Ll][\*_~|]*").unwrap();
    }

    if let Some(c) = VOTE_RE.captures(content.as_str()) {
        return Some(Vote::VTL(capitalize(c.get(1).map_or("", |m| m.as_str()))));
    };

    if let Some(c) = UN_VOTE_RE.captures(content.as_str()) {
        return Some(Vote::UnVTL(capitalize(c.get(1).map_or("", |m| m.as_str()))));
    };

    if VTNL_RE.is_match(content.as_str()) {
        Some(Vote::VTNL)
    } else {
        None
    }
}

fn get_non_voters<'a>(players: Vec<&'a User>, votes: &mut HashMap<&'a User, String>) {
    for player in players {
        if !votes.contains_key(player) {
            votes.insert(player, String::from("No vote"));
        }
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

    let can_change_na = match data.can_change_na {
        Some(c) => c,
        None => true,
    };

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

    let items = get_items(client, input).await.unwrap_or_default();

    let mut desc = String::new();
    if !items.is_empty() {
        if items.len() == 1 {
            msg.channel_id.say(&ctx.http, items[0].url.clone()).await?;
            return Ok(());
        }
        for (idx, item) in items.iter().enumerate() {
            let _ = write!(desc, "\n{}. [{}]({})", idx + 1, item.title, item.url);
        }
    } else {
        msg.channel_id.say(&ctx.http, "No results found.").await?;
        return Ok(());
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
/// **Example**
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
                        If it has, please use a host to use the `started` command.\
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
                Vote::VTL(u) => write!(votes_str, "\n{}. **VTL {}**", count, u),
                Vote::UnVTL(u) => write!(votes_str, "\n{}. **UnVTL {}**", count, u),
                Vote::VTNL => write!(votes_str, "\n{}. **VTNL**", count),
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
    vote_history
)]
#[description("General commands for users.")]
struct UserCommands;
