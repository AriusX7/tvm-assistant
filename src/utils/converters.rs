// Functions and wrappers to get `Role`, Member`, etc., from
// user input and/or database values.

use serenity::{
    model::{channel::GuildChannel, prelude::*},
    prelude::*,
    utils::parse_mention,
};
use std::{borrow::Cow, collections::HashMap};

pub fn search_role_name(roles: &HashMap<RoleId, Role>, name: &str) -> Option<Role> {
    roles.values().find(|r| r.name == name).cloned()
}

pub async fn to_role(ctx: &Context, guild_id: GuildId, input: &str) -> Option<Role> {
    let roles = ctx.cache.guild_field(guild_id, |g| g.roles.clone()).await?;

    match input.parse::<u64>() {
        Ok(i) => roles.get(&RoleId(i)).cloned(),
        Err(_) => match parse_mention(input) {
            Some(i) => roles.get(&RoleId(i)).cloned(),
            None => search_role_name(&roles, input),
        },
    }
}

pub fn search_channel_name(
    channels: &HashMap<ChannelId, GuildChannel>,
    name: &str,
) -> Option<GuildChannel> {
    channels.values().find(|c| c.name == name).cloned()
}

pub async fn to_channel(ctx: &Context, guild_id: GuildId, input: &str) -> Option<GuildChannel> {
    let channels = ctx
        .cache
        .guild_field(guild_id, |g| g.channels.clone())
        .await?;

    match input.parse::<u64>() {
        Ok(i) => channels.get(&ChannelId(i)).cloned(),
        Err(_) => match parse_mention(input) {
            Some(i) => channels.get(&ChannelId(i)).cloned(),
            None => search_channel_name(&channels, input),
        },
    }
}

/// Wrapper around `to_role`.
pub async fn get_role(
    ctx: &Context,
    guild_id: GuildId,
    input: Option<i64>,
) -> Result<Role, &'static str> {
    match input {
        Some(i) => {
            if let Some(r) = to_role(ctx, guild_id, &i.to_string()).await {
                Ok(r)
            } else {
                Err("No role was found from the given input.")
            }
        }
        None => Err("No role was found from the given input."),
    }
}

pub fn search_member_name(members: &HashMap<UserId, Member>, name: &str) -> Option<Member> {
    let name = Cow::from(name);
    members
        .values()
        .find(|m| m.display_name() == name || m.user.name == name)
        .cloned()
}

pub async fn to_member(ctx: &Context, guild_id: GuildId, input: &str) -> Option<Member> {
    let members = ctx
        .cache
        .guild_field(guild_id, |g| g.members.clone())
        .await?;

    match input.parse::<u64>() {
        Ok(i) => members.get(&UserId(i)).cloned(),
        Err(_) => match parse_mention(input) {
            Some(i) => members.get(&UserId(i)).cloned(),
            None => search_member_name(&members, input),
        },
    }
}

/// Wrapper around `to_member`.
pub async fn get_member(
    ctx: &Context,
    guild_id: GuildId,
    input: Option<&String>,
) -> Result<Member, &'static str> {
    match input {
        Some(i) => {
            if let Some(m) = to_member(ctx, guild_id, &i).await {
                Ok(m)
            } else {
                Err("No member was found from the given input.")
            }
        }
        None => Err("No member was found from the given input."),
    }
}

#[allow(unused)]
/// Wrapper around `to_channel`.
pub async fn get_channel(
    ctx: &Context,
    guild_id: GuildId,
    input: Option<&String>,
) -> Result<GuildChannel, &'static str> {
    match input {
        Some(i) => {
            if let Some(c) = to_channel(ctx, guild_id, &i).await {
                Ok(c)
            } else {
                Err("No channel was found from the given input.")
            }
        }
        None => Err("No channel was found from the given input."),
    }
}

/// Wrapper around `to_member` to get channel from id.
pub async fn get_channel_from_id(
    ctx: &Context,
    guild_id: GuildId,
    input: Option<i64>,
) -> Result<GuildChannel, &'static str> {
    // We can use the same structure as `get_channel` because `to_channel`
    // checks for channel by `ID` first, so the function is *short-circuited*.
    match input {
        Some(i) => {
            if let Some(c) = to_channel(ctx, guild_id, &i.to_string()).await {
                Ok(c)
            } else {
                Err("No channel was found from the given input.")
            }
        }
        None => Err("No channel was found from the given input."),
    }
}
