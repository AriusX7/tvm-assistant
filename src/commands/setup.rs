// Commands related to TvM's setup are defined here.

use crate::{
    utils::{
        checks::*, constants::EMBED_COLOUR, converters::*, database::initialize_tables,
        predicates::yes_or_no_pred,
    },
    ConnectionPool,
};
use log::error;
use serde::{Deserialize, Serialize};
use serenity::{
    framework::standard::{
        macros::{command, group},
        Args, CommandError, CommandResult,
    },
    model::{misc::Mentionable, prelude::*},
    prelude::*,
};
use sqlx::types::Json;
use std::fmt::Write;

#[derive(Deserialize)]
pub struct Settings {
    pub guild_id: i64,
    pub host_role_id: Option<i64>,
    pub player_role_id: Option<i64>,
    pub spec_role_id: Option<i64>,
    pub repl_role_id: Option<i64>,
    pub dead_role_id: Option<i64>,
    pub na_channel_id: Option<i64>,
    pub signups_channel_id: Option<i64>,
    pub can_change_na: Option<bool>,
    pub tvmset_lock: Option<bool>,
    pub signups_on: Option<bool>,
    pub total_players: Option<i16>,
    pub total_signups: Option<i16>,
    pub na_submitted: Option<Vec<i64>>,
    pub cycle: Option<Json<Cycle>>,
}

#[derive(Deserialize, Serialize)]
pub struct Cycle {
    pub number: i16,
    pub day: Option<i64>,
    pub night: Option<i64>,
    pub votes: Option<i64>,
}

/// Sets the Host role.
///
/// **Usage:** `[p]host <role>`
///
/// The bot asks for confirmation before making any changes. This command
/// cannot be used if the TvM settings are locked.
#[command("host")]
#[checks("tvmset_lock")]
pub async fn host_role(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    set_role(ctx, msg, args, "Host", "host_role_id").await
}

/// Sets the Player role.
///
/// **Usage:** `[p]player <role>`
///
/// The bot asks for confirmation before making any changes. This command
/// cannot be used if the TvM settings are locked.
#[command("player")]
#[checks("tvmset_lock")]
pub async fn player_role(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    set_role(ctx, msg, args, "Player", "player_role_id").await
}

/// Sets the Spectator role.
///
/// **Usage:** `[p]spec <role>`
///
/// **Alias:** `spectator`
///
/// The bot asks for confirmation before making any changes. This command
/// cannot be used if the TvM settings are locked.
#[command("spec")]
#[aliases("spectator")]
#[checks("tvmset_lock")]
pub async fn spectator_role(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    set_role(ctx, msg, args, "Spectator", "spec_role_id").await
}

/// Sets the Replacement role.
///
/// **Usage:** `[p]repl <role>`
///
/// **Alias:** `replacement`
///
/// The bot asks for confirmation before making any changes. This command
/// cannot be used if the TvM settings are locked.
#[command("repl")]
#[aliases("replacement")]
#[checks("tvmset_lock")]
pub async fn repl_role(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    set_role(ctx, msg, args, "Replacement", "repl_role_id").await
}

/// Sets the Dead player role.
///
/// **Usage:** `[p]dead <role>`
///
/// The bot asks for confirmation before making any changes. This command
/// cannot be used if the TvM settings are locked.
#[command("dead")]
#[checks("tvmset_lock")]
pub async fn dead_role(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    set_role(ctx, msg, args, "Dead", "dead_role_id").await
}

/// Automatically creates and sets Host, Player, Specator, Replacement and Dead roles.
///
/// **Usage:** `[p]setroles`
///
/// The bot adds the Host role to the person using the command if it has the permissions.
///
/// This command cannot be used if the TvM settings are locked.
#[command("setroles")]
#[checks("tvmset_lock")]
pub async fn set_all_roles(ctx: &Context, msg: &Message) -> CommandResult {
    // Get guild, then create 5 roles.
    let guild = match msg.guild(ctx).await {
        Some(i) => i,
        None => return Err(CommandError::from("Couldn't fetch details of this server.")),
    };

    let host_role = match guild
        .create_role(ctx, |r| {
            r.hoist(true)
                .mentionable(true)
                .name("Hosts")
                .colour(0xFFBF37)
        })
        .await
    {
        Ok(r) => r,
        Err(_) => {
            msg.channel_id
                .say(&ctx.http, "Could not create role.")
                .await?;
            return Ok(());
        }
    };

    if let Some(m) = guild.members.get(&msg.author.id) {
        // Try to add the role, do nothing if unable to.
        let _ = m.clone().add_role(&ctx.http, host_role.id).await;
    };

    // If we're able to create a role, then we can assume we will be able to create the remaining
    // four. This is why we let serenity handle the error by using `?`.
    let player_role = guild
        .create_role(ctx, |r| {
            r.hoist(true)
                .mentionable(true)
                .name("Players")
                .colour(0x37BFFF)
        })
        .await?;
    let repl_role = guild
        .create_role(ctx, |r| {
            r.hoist(true)
                .mentionable(true)
                .name("Replacements")
                .colour(0x86FF40)
        })
        .await?;
    let spec_role = guild
        .create_role(ctx, |r| {
            r.hoist(true)
                .mentionable(true)
                .name("Spectators")
                .colour(0xD837FF)
        })
        .await?;
    let dead_role = guild
        .create_role(ctx, |r| {
            r.hoist(true)
                .mentionable(true)
                .name("Dead")
                .colour(0xDC5757)
        })
        .await?;

    // Put them all in the database.

    let data_read = ctx.data.read().await;
    let pool = data_read.get::<ConnectionPool>().unwrap();

    sqlx::query!(
        "
        INSERT INTO config (
            guild_id,
            host_role_id,
            player_role_id,
            spec_role_id,
            repl_role_id,
            dead_role_id
        ) VALUES (
            $1, $2, $3, $4, $5, $6
        ) ON CONFLICT (guild_id) DO UPDATE SET
            host_role_id = $2,
            player_role_id = $3,
            spec_role_id = $4,
            repl_role_id = $5,
            dead_role_id = $6
        ",
        guild.id.0 as i64,
        host_role.id.0 as i64,
        player_role.id.0 as i64,
        spec_role.id.0 as i64,
        repl_role.id.0 as i64,
        dead_role.id.0 as i64
    )
    .execute(pool)
    .await?;

    match &msg
        .channel_id
        .send_message(&ctx.http, |m| {
            m.embed(|e| {
                e.colour(0x37BFFF);
                e.description(format!(
                    "Host: {}\nPlayer: {}\nSpectator: {}\nReplacement: {}\nDead: {}",
                    host_role.mention(),
                    player_role.mention(),
                    spec_role.mention(),
                    repl_role.mention(),
                    dead_role.mention()
                ));
                e.title("Created Roles");

                e
            });

            m
        })
        .await
    {
        Ok(_) => (),
        Err(_) => {
            // Assuming there was an error sending the embed.
            // We'll send the message in text form, and let serenity handle if there is an error.
            msg.channel_id
                .say(
                    &ctx.http,
                    format!(
                        "
                    **Created Roles:**\
                    \n\nHost: `{}`\
                    \nPlayer: `{}`\
                    \nSpectator: `{}\
                    `\nReplacement: `{}`\
                    \nDead: `{}`
                    ",
                        host_role.name,
                        player_role.name,
                        spec_role.name,
                        repl_role.name,
                        dead_role.name
                    ),
                )
                .await?;
        }
    };

    Ok(())
}

async fn set_role(
    ctx: &Context,
    msg: &Message,
    args: Args,
    role_type: &str,
    database_spec: &str,
) -> CommandResult {
    let guild_id = match msg.guild_id {
        Some(i) => i,
        None => {
            return Err(CommandError::from(
                "There was an error getting this server",
            ))
        }
    };
    let input = match args.remains() {
        Some(i) => i,
        None => {
            msg.channel_id
                .say(&ctx.http, "I need an argument to run this command.")
                .await?;
            return Ok(());
        }
    };

    let role = match to_role(ctx, guild_id, input).await {
        Some(r) => r,
        None => {
            return Err(CommandError::from(format!(
                "Role with name **{}** was not found.",
                input
            )))
        }
    };

    let confirmation_msg = &msg
        .channel_id
        .say(
            &ctx.http,
            format!(
                "Are you sure you want to set **{}** as {} role?",
                role.name, role_type
            ),
        )
        .await?;

    match yes_or_no_pred(&ctx, &msg, &confirmation_msg).await {
        Ok(c) if !c => {
            msg.channel_id
                .say(&ctx.http, format!("{} role setup cancelled.", role_type))
                .await?;
            return Ok(());
        }
        Ok(_) => (),
        Err(e) => return Err(e),
    };

    let data_read = ctx.data.read().await;
    let pool = data_read.get::<ConnectionPool>().unwrap();

    // let query_role_type = format!("{}_role_id", role_type.to_ascii_lowercase());
    let query = format!(
        "
        INSERT INTO config(guild_id, {0}) VALUES($1, $2)
        ON CONFLICT (guild_id) DO UPDATE SET {0} = $2;
        ",
        database_spec
    );

    sqlx::query(query.as_str())
        .bind(guild_id.0 as i64)
        .bind(role.id.0 as i64)
        .execute(pool)
        .await?;

    msg.channel_id
        .say(
            &ctx.http,
            format!("Set `{}` as {} role!", role.name, role_type),
        )
        .await?;

    Ok(())
}

/// Sets the Night Actions channel.
///
/// **Usage:** `[p]nachannel <channel>`
///
/// All night action messages sent by a user are sent to this channel.
///
/// The bot asks for confirmation before making any changes. This command
/// cannot be used if the TvM settings are locked.
#[command("nachannel")]
#[checks("tvmset_lock")]
pub async fn na_channel(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    set_channel(ctx, msg, args, "Night Actions", "na_channel_id").await
}

/// Sets the sign-ups channel.
///
/// **Usage:** `[p]signups <channel>`
///
/// Users can only sign-up (as player, spectator or replacement) in this channel.
///
/// The bot asks for confirmation before making any changes. This command
/// cannot be used if the TvM settings are locked.
#[command("signups")]
#[checks("tvmset_lock")]
pub async fn signups_channel(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    set_channel(ctx, msg, args, "Sign-ups", "signups_channel_id").await
}

/// Creates and sets night actions and sign-ups channels.
///
/// **Usage:** `[p]setchannels`
///
/// This command cannot be used if the TvM settings are locked.
#[command("setchannels")]
#[checks("tvmset_lock")]
pub async fn set_all_channels(ctx: &Context, msg: &Message) -> CommandResult {
    // Get guild, then create 5 roles.
    let guild = match msg.guild(ctx).await {
        Some(i) => i,
        None => return Err(CommandError::from("Couldn't fetch details of this server.")),
    };

    let signups_perms = vec![
        PermissionOverwrite {
            allow: Permissions::READ_MESSAGES,
            deny: Permissions::SEND_TTS_MESSAGES,
            kind: PermissionOverwriteType::Role(RoleId(guild.id.0)),
        },
        PermissionOverwrite {
            allow: Permissions::ADD_REACTIONS,
            deny: Permissions::empty(),
            kind: PermissionOverwriteType::Member(ctx.cache.current_user().await.id),
        },
    ];

    let signups = match guild
        .create_channel(ctx, |c| {
            c.name("sign-ups")
                .kind(ChannelType::Text)
                .permissions(signups_perms)
        })
        .await
    {
        Ok(r) => r,
        Err(_) => {
            msg.channel_id
                .say(&ctx.http, "Could not create channel.")
                .await?;
            return Ok(());
        }
    };

    let mut na_perms = vec![
        PermissionOverwrite {
            allow: Permissions::empty(),
            deny: Permissions::READ_MESSAGES,
            kind: PermissionOverwriteType::Role(RoleId(guild.id.0)),
        },
        PermissionOverwrite {
            allow: Permissions::READ_MESSAGES | Permissions::SEND_MESSAGES,
            deny: Permissions::empty(),
            kind: PermissionOverwriteType::Member(ctx.cache.current_user().await.id),
        },
    ];

    // Get host role if it exists, and then allow hosts to see NA channel.
    let data_read = ctx.data.read().await;
    let pool = data_read.get::<ConnectionPool>().unwrap();

    if let Ok(r) = sqlx::query!(
        "SELECT host_role_id FROM config WHERE guild_id = $1",
        guild.id.0 as i64
    )
    .fetch_one(pool)
    .await
    {
        if let Some(i) = r.host_role_id {
            if to_role(ctx, guild.id, &i.to_string()).await.is_some() {
                na_perms.push(PermissionOverwrite {
                    allow: Permissions::READ_MESSAGES,
                    deny: Permissions::empty(),
                    kind: PermissionOverwriteType::Role(RoleId(i as u64)),
                });
            };
        }
    }

    // If we're able to create a channel, then we can assume we will be able to create the other.
    // This is why we let serenity handle the error by using `?`.
    let na_channel = guild
        .create_channel(ctx, |c| {
            c.name("night-actions")
                .kind(ChannelType::Text)
                .permissions(na_perms)
        })
        .await?;

    // Put them both in the database.

    sqlx::query!(
        "
        INSERT INTO config (
            guild_id,
            signups_channel_id,
            na_channel_id
        ) VALUES (
            $1, $2, $3
        ) ON CONFLICT (guild_id) DO UPDATE SET
        signups_channel_id = $2,
        na_channel_id = $3
        ",
        guild.id.0 as i64,
        signups.id.0 as i64,
        na_channel.id.0 as i64
    )
    .execute(pool)
    .await?;

    match &msg
        .channel_id
        .send_message(&ctx.http, |m| {
            m.embed(|e| {
                e.colour(0x37BFFF);
                e.description(format!(
                    "Sign-ups: {}\nNight Actions: {}\n",
                    signups.mention(),
                    na_channel.mention(),
                ));
                e.title("Created Channels");

                e
            });

            m
        })
        .await
    {
        Ok(_) => (),
        Err(_) => {
            // Assuming there was an error sending the embed.
            // We'll send the message in text form, and let serenity handle if there is an error.
            msg.channel_id
                .say(
                    &ctx.http,
                    format!(
                        "
                    **Created Channels:**\
                    \n\nSign-ups: {}\
                    \nNight Actions: {}\
                    ",
                        signups.mention(),
                        na_channel.mention()
                    ),
                )
                .await?;
        }
    };

    Ok(())
}

async fn set_channel(
    ctx: &Context,
    msg: &Message,
    args: Args,
    channel_type: &str,
    database_spec: &str,
) -> CommandResult {
    let guild_id = match msg.guild_id {
        Some(i) => i,
        None => {
            return Err(CommandError::from(
                "There was an error getting this server.",
            ))
        }
    };
    let input = match args.remains() {
        Some(i) => i,
        None => {
            msg.channel_id
                .say(&ctx.http, "I need an argument to run this command.")
                .await?;
            return Ok(());
        }
    };

    let channel = match to_channel(ctx, guild_id, input).await {
        Some(c) => {
            if let ChannelType::Text = c.kind {
                c
            } else {
                return Err(CommandError::from(format!(
                    "{} is not a text channel.",
                    c.mention()
                )));
            }
        }
        None => {
            return Err(CommandError::from(format!(
                "Channel with name **{}** was not found.",
                input
            )))
        }
    };

    let confirmation_msg = &msg
        .channel_id
        .say(
            &ctx.http,
            format!(
                "Are you sure you want to set {} as {} channel?",
                channel.mention(),
                channel_type
            ),
        )
        .await?;

    match yes_or_no_pred(&ctx, &msg, &confirmation_msg).await {
        Ok(c) if !c => {
            msg.channel_id
                .say(
                    &ctx.http,
                    format!("{} channel setup cancelled.", channel_type),
                )
                .await?;
            return Ok(());
        }
        Ok(_) => (),
        Err(e) => return Err(e),
    };

    let data_read = ctx.data.read().await;
    let pool = data_read.get::<ConnectionPool>().unwrap();

    // let query_role_type = format!("{}_role_id", role_type.to_ascii_lowercase());
    let query = format!(
        "
        INSERT INTO config(guild_id, {0}) VALUES($1, $2)
        ON CONFLICT (guild_id) DO UPDATE SET {0} = $2;
        ",
        database_spec
    );

    sqlx::query(query.as_str())
        .bind(guild_id.0 as i64)
        .bind(channel.id.0 as i64)
        .execute(pool)
        .await?;

    msg.channel_id
        .say(
            &ctx.http,
            format!("Set {} as {} channel!", channel.mention(), channel_type),
        )
        .await?;

    Ok(())
}

/// Toggles the `Can Change NA` setting. It is `true` by default.
///
/// **Usage:** `[p]changena [setting]`
///
/// You can optionally specify the setting to use. `setting` can be one of
/// - `true`
/// - `false`
///
/// This command cannot be used if the TvM settings are locked.
#[command("changena")]
#[description = "Toggles `Can Change NA` setting. `true` by default."]
#[checks("tvmset_lock")]
pub async fn can_change_na(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let mut toggle = false;
    // setting = true by default
    let mut setting = true;

    if args.is_empty() {
        toggle = true;
    } else {
        setting = match args.single() {
            Ok(s) => s,
            Err(_) => return Err(CommandError::from("Invalid option.")),
        };
    }

    let data_read = ctx.data.read().await;
    let pool = data_read.get::<ConnectionPool>().unwrap();

    if toggle {
        if let Ok(r) = sqlx::query!(
            "SELECT can_change_na FROM config WHERE guild_id = $1",
            msg.guild_id.unwrap().0 as i64
        )
        .fetch_one(pool)
        .await
        {
            if let Some(current) = r.can_change_na {
                setting = !current;
            }
        }
    }

    sqlx::query!(
        "
        INSERT INTO config(guild_id, can_change_na) VALUES($1, $2)
        ON CONFLICT (guild_id) DO UPDATE SET can_change_na = $2;
        ",
        msg.guild_id.unwrap().0 as i64,
        setting
    )
    .execute(pool)
    .await?;

    msg.channel_id
        .say(&ctx.http, format!("Set `Can Change NA` to {}.", setting))
        .await?;

    Ok(())
}

/// Sets the maximum number of players allowed to sign-up.
///
/// **Usage:** `[p]total <number>`
///
/// The total is `12` by default.
///
/// This command cannot be used if the TvM settings are locked.
#[command("total")]
#[checks("tvmset_lock")]
pub async fn total_players(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let total: i16 = match args.single() {
        Ok(i) => i,
        Err(_) => {
            msg.channel_id
                .say(&ctx.http, "`total` must be a valid number.")
                .await?;
            return Ok(());
        }
    };

    if total < 0 {
        msg.channel_id
            .say(&ctx.http, "`total` must be a positive number.")
            .await?;
        return Ok(());
    }

    let data_read = ctx.data.read().await;
    let pool = data_read.get::<ConnectionPool>().unwrap();

    sqlx::query!(
        "
        INSERT INTO config(guild_id, total_players) VALUES($1, $2)
        ON CONFLICT (guild_id) DO UPDATE SET total_players = $2;
        ",
        msg.guild_id.unwrap().0 as i64,
        total
    )
    .execute(pool)
    .await?;

    msg.channel_id
        .say(
            &ctx.http,
            format!("Set total number of players to {}.", total),
        )
        .await?;

    Ok(())
}

/// Opens sign-ups.
///
/// **Usage:** `[p]signopen`
///
/// Sign-ups are open by default.
///
/// This command cannot be used if the TvM settings are locked.
#[command("signopen")]
#[checks("tvmset_lock")]
pub async fn sign_open(ctx: &Context, msg: &Message) -> CommandResult {
    let data_read = ctx.data.read().await;
    let pool = data_read.get::<ConnectionPool>().unwrap();

    sqlx::query!(
        "
        INSERT INTO config(guild_id, signups_on) VALUES($1, $2)
        ON CONFLICT (guild_id) DO UPDATE SET signups_on = $2;
        ",
        msg.guild_id.unwrap().0 as i64,
        true
    )
    .execute(pool)
    .await?;

    msg.channel_id.say(&ctx.http, "Opened sign-ups.").await?;

    Ok(())
}

/// Closes sign-ups.
///
/// **Usage:** `[p]signclose`
///
/// Sign-ups are open by default.
///
/// This command cannot be used if the TvM settings are locked.
#[command("signclose")]
#[checks("tvmset_lock")]
pub async fn sign_close(ctx: &Context, msg: &Message) -> CommandResult {
    let data_read = ctx.data.read().await;
    let pool = data_read.get::<ConnectionPool>().unwrap();

    sqlx::query!(
        "
        INSERT INTO config(guild_id, signups_on) VALUES($1, $2)
        ON CONFLICT (guild_id) DO UPDATE SET signups_on = $2;
        ",
        msg.guild_id.unwrap().0 as i64,
        false
    )
    .execute(pool)
    .await?;

    msg.channel_id.say(&ctx.http, "Closed sign-ups.").await?;

    Ok(())
}

/// Locks the TvM settings.
///
/// **Usage:** `[p]lock`
///
/// Commands that can change TvM settings cannot be used after locking.
/// You can unlock the commands by using `[p]unlock` command.
///
/// **It is recommened to lock the settings before starting the game.**
///
/// This command cannot be used if the TvM settings are locked.
#[command("lock")]
#[checks("tvmset_lock")]
pub async fn lock_settings(ctx: &Context, msg: &Message) -> CommandResult {
    let data_read = ctx.data.read().await;
    let pool = data_read.get::<ConnectionPool>().unwrap();

    sqlx::query!(
        "
        INSERT INTO config(guild_id, tvmset_lock) VALUES($1, $2)
        ON CONFLICT (guild_id) DO UPDATE SET tvmset_lock = $2;
        ",
        msg.guild_id.unwrap().0 as i64,
        true
    )
    .execute(pool)
    .await?;

    msg.channel_id
        .say(&ctx.http, "Locked TvM settings.")
        .await?;

    Ok(())
}

/// Unlocks the TvM settings.
///
/// **Usage:** `[p]unlock`
///
/// Commands are *unlocked* by default.
#[command("unlock")]
#[description = "Unlocks TvM settings configuration."]
pub async fn unlock_settings(ctx: &Context, msg: &Message) -> CommandResult {
    let data_read = ctx.data.read().await;
    let pool = data_read.get::<ConnectionPool>().unwrap();

    sqlx::query!(
        "
        INSERT INTO config(guild_id, tvmset_lock) VALUES($1, $2)
        ON CONFLICT (guild_id) DO UPDATE SET tvmset_lock = $2;
        ",
        msg.guild_id.unwrap().0 as i64,
        false
    )
    .execute(pool)
    .await?;

    msg.channel_id
        .say(&ctx.http, "Unlocked TvM settings.")
        .await?;

    Ok(())
}

/// Shows the TvM settings.
///
/// **Usage:** `[p]settings`
///
/// **Alias:** `show`
///
/// Embed Links permission is required for this command to work.
#[command("settings")]
#[aliases("show")]
pub async fn tvm_settings(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = match msg.guild(ctx).await {
        Some(i) => i,
        None => return Err(CommandError::from("Couldn't fetch details of this server.")),
    };

    let data_read = ctx.data.read().await;
    let pool = data_read.get::<ConnectionPool>().unwrap();

    let settings: Settings = match sqlx::query_as_unchecked!(
        Settings,
        "
        SELECT * FROM config WHERE guild_id = $1;
        ",
        guild.id.0 as i64
    )
    .fetch_one(pool)
    .await
    {
        Ok(s) => s,
        Err(_) => {
            error!(
                "config table wasn't initialized for guild with ID {}.",
                guild.id.0 as i64
            );
            // Initialize all three tables.
            initialize_tables(&ctx, &guild).await;
            return Ok(());
        }
    };

    let mut fields = Vec::new();

    let mut roles_str = String::new();
    match settings.host_role_id {
        Some(id) => write!(roles_str, "\nHost Role: <@&{}>", id),
        None => write!(roles_str, "\nHost Role: `Not set`"),
    }?;
    match settings.player_role_id {
        Some(id) => write!(roles_str, "\nPlayer Role: <@&{}>", id),
        None => write!(roles_str, "\nPlayer Role: `Not set`"),
    }?;
    match settings.spec_role_id {
        Some(id) => write!(roles_str, "\nSpectator Role: <@&{}>", id),
        None => write!(roles_str, "\nSpectator Role: `Not set`"),
    }?;
    match settings.repl_role_id {
        Some(id) => write!(roles_str, "\nReplacement Role: <@&{}>", id),
        None => write!(roles_str, "\nReplacement Role: `Not set`"),
    }?;
    match settings.dead_role_id {
        Some(id) => write!(roles_str, "\nDead Player Role: <@&{}>", id),
        None => write!(roles_str, "\nDead Player Role: `Not set`"),
    }?;

    fields.push(("**Roles**", roles_str.trim(), false));

    let mut channels_str = String::new();
    match settings.signups_channel_id {
        Some(id) => write!(channels_str, "\nSign-ups Channel: <#{}>", id),
        None => write!(channels_str, "\nSign-ups Channel: `Not set`"),
    }?;
    match settings.na_channel_id {
        Some(id) => write!(channels_str, "\nNight Actions Channel: <#{}>", id),
        None => write!(channels_str, "\nNight Actions Channel: `Not set`"),
    }?;

    fields.push(("**Channels**", channels_str.trim(), false));

    let mut misc_str = String::new();
    match settings.tvmset_lock {
        Some(b) => write!(misc_str, "\nTvM Settings Lock: `{}`", b),
        None => write!(misc_str, "\nTvM Settings Lock: `false`"),
    }?;
    match settings.can_change_na {
        Some(b) => write!(misc_str, "\nCan Change Night Action: `{}`", b),
        None => write!(misc_str, "\nCan Change Night Action: `true`"),
    }?;
    match settings.signups_on {
        Some(b) if !b => write!(misc_str, "\nSign-ups: `Closed`"),
        _ => write!(misc_str, "\nSign-ups: `Open`"),
    }?;
    match settings.total_players {
        Some(t) => write!(misc_str, "\nMaximum Players: `{}`", t),
        None => write!(misc_str, "\nMaximum Players: `12`"),
    }?;

    fields.push(("**Miscellaneous**", misc_str.trim(), false));

    msg.channel_id
        .send_message(&ctx.http, |m| {
            m.embed(|e| {
                e.title("TvM Settings");
                e.colour(EMBED_COLOUR);
                e.fields(fields);

                e
            });

            m
        })
        .await?;

    Ok(())
}

#[group("TvM Settings")]
#[prefix = "tvm"]
#[checks("is_host_or_admin")]
#[only_in("guilds")]
#[commands(
    host_role,
    player_role,
    spectator_role,
    repl_role,
    dead_role,
    na_channel,
    signups_channel,
    can_change_na,
    total_players,
    sign_open,
    sign_close,
    lock_settings,
    unlock_settings,
    tvm_settings,
    set_all_roles,
    set_all_channels
)]
#[default_command(tvm_settings)]
#[description("Commands for hosts to set TvM settings.")]
struct TvMSet;
