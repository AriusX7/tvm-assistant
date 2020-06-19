// This module contains commands and related functiosn for general
// users, like players, spectators and replacements.

use crate::{
    commands::{
        host::{get_na_channel, Data},
        setup::Cycle,
    },
    utils::{
        converters::{get_channel, get_channel_from_id, get_role, to_channel, to_role},
        formatting::{capitalize, clean_user_mentions},
    },
    ConnectionPool,
};
use chrono::{offset::Utc, Duration};
use indexmap::IndexMap;
use regex::Regex;
use serenity::{
    framework::standard::{
        macros::{command, group},
        Args, CommandError, CommandResult,
    },
    model::{
        misc::Mentionable,
        prelude::{Guild, GuildChannel, Member, Message, PermissionOverwriteType, Role, User},
    },
    prelude::Context,
    utils::{content_safe, ContentSafeOptions},
};
use sqlx::types::Json;
use std::{borrow::Cow, collections::HashMap, fmt::Write};

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
/// Sign-out from the TvM or sign-up as spectator.
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

    let mut all_messages = match channel.messages(&ctx.http, |ret| ret.limit(100)).await {
        Ok(m) => m,
        Err(_) => return Err(CommandError::from("I was unable to get messages.")),
    };
    // Messages are ordered from new to oldest. We need to reverse that.
    all_messages.reverse();

    // Time to filter messages to only keep those sent by a player.

    let members = &guild.members;
    // Instead of filtering, chec
    let messages = all_messages
        .iter()
        .filter(|m| match members.get(&m.author.id) {
            Some(m) => m.roles.contains(&role.id),
            None => false,
        });

    let mut user_votes = HashMap::new();
    for message in messages {
        user_votes.insert(
            &message.author,
            get_vote_from_message(clean_user_mentions(&message)),
        );
    }

    // Adds non-voters to `user_votes`.
    get_non_voters(&guild, &role, &mut user_votes);

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

fn get_vote_from_message(content: String) -> String {
    lazy_static! {
        static ref VOTE_RE: Regex =
            Regex::new(r"[\*_~|]*[Vv][Tt][Ll][\*_~|]* ([^\s\*_~|]+)").unwrap();
        static ref UN_VOTE_RE: Regex =
            Regex::new(r"[\*_~|]*[Uu][Nn]-?[Vv][Tt][Ll][\*_~|]*\s?([^\s\*_~|]+)?").unwrap();
        static ref VTNL_RE: Regex = Regex::new(r"[\*_~|]*[Vv][Tt][Nn][Ll][\*_~|]*").unwrap();
    }

    if let Some(c) = VOTE_RE.captures(content.as_str()) {
        return capitalize(c.get(1).unwrap().as_str());
    };

    if let Some(c) = UN_VOTE_RE.captures(content.as_str()) {
        return capitalize(c.get(1).unwrap().as_str());
    };

    if VTNL_RE.is_match(content.as_str()) {
        capitalize("VTNL")
    } else {
        capitalize("No vote")
    }
}

fn get_non_voters<'a>(guild: &'a Guild, player_role: &Role, votes: &mut HashMap<&'a User, String>) {
    for player in guild
        .members
        .values()
        .filter(|m| m.roles.contains(&player_role.id))
    {
        if !votes.contains_key(&player.user) {
            votes.insert(&player.user, String::from("No vote"));
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
    night_action
)]
#[description("General commands for users.")]
struct UserCommands;
