// All host utility commands and related functions are defined here.

use crate::{
    commands::setup::Cycle,
    utils::{checks::*, converters::*, predicates::yes_or_no_pred},
    ConnectionPool,
};
use rand::Rng;
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

pub(crate) struct Data {
    pub(crate) player_role_id: Option<i64>,
    pub(crate) cycle: Option<Json<Cycle>>,
}

/// Randomly assigns a role from a comma-separated list to a player.
///
/// **Usage:** `[p]rand <role_1[, role_2[, ...]]>`
///
/// **Aliases:** `randomise`, `randomize`
///
/// Player role must be set to use this command. Additionally, the number of
/// roles in the list must be exactly equal to number of members with the Player role.
///
/// You can specify one role multiple times.
///
/// **Example**
///
/// *Assuming `Arius`, `Ligi`, and `Craw` have Player role.*
///
/// Command: `[p]rand doctor, mafioso, jailor`
///
/// Output:
/// ```
/// Arius: jailor
/// Ligi: mafioso
/// Craw: doctor
/// ```
#[command("rand")]
#[aliases("randomise", "randomize")]
#[min_args(1)]
async fn randomize_roles(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    // Pre-invocation check will make sure `args` isn't empty.
    let mut args: Vec<&str> = args
        .message()
        .split_terminator(',')
        .filter_map(|x| {
            if !x.trim().is_empty() {
                Some(x.trim())
            } else {
                None
            }
        })
        .collect();

    let guild = match msg.guild(ctx).await {
        Some(i) => i,
        None => return Err(CommandError::from("Couldn't fetch details of this server.")),
    };

    let data_read = ctx.data.read().await;
    let pool = data_read.get::<ConnectionPool>().unwrap();

    let res = sqlx::query!(
        "SELECT player_role_id FROM config WHERE guild_id = $1",
        guild.id.0 as i64
    )
    .fetch_one(pool)
    .await?;

    let role = match get_role(ctx, guild.id, res.player_role_id).await {
        Ok(r) => r,
        Err(_) => {
            msg.channel_id
                .say(&ctx.http, "Player role has not been set up.")
                .await?;
            return Ok(());
        }
    };

    let players: Vec<&Member> = guild
        .members
        .values()
        .filter(|m| m.roles.contains(&role.id))
        .collect();
    if players.len() != args.len() {
        msg.channel_id
            .say(
                &ctx.http,
                "Number of members with `Player` role is not equal to number of roles.",
            )
            .await?;
        return Ok(());
    }

    // let mut assigned_roles = HashMap::new();
    let mut assigned_roles = String::new();
    for player in players {
        // We can unwrap here safely because lengths are equal.
        let mut rng = rand::thread_rng();
        let index = rng.gen_range(0, args.len());

        write!(
            assigned_roles,
            "\n{}: {}",
            player.display_name(),
            args[index]
        )?;

        args.remove(index);
    }

    msg.channel_id.say(&ctx.http, assigned_roles.trim()).await?;

    Ok(())
}

/// Syncs total sign-ups with number of members with Player role.
///
/// **Usage:** `[p]synctotal`
#[command("synctotal")]
async fn sync_total(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = match msg.guild(ctx).await {
        Some(i) => i,
        None => return Err(CommandError::from("Couldn't fetch details of this server.")),
    };

    let data_read = ctx.data.read().await;
    let pool = data_read.get::<ConnectionPool>().unwrap();

    let res = match sqlx::query!(
        "
        SELECT total_signups, player_role_id FROM config WHERE guild_id = $1;
        ",
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

    let total_signups = res.total_signups.unwrap_or(0);

    let role = match get_role(ctx, guild.id, res.player_role_id).await {
        Ok(r) => r,
        Err(_) => {
            msg.channel_id
                .say(&ctx.http, "Player role has not been set up.")
                .await?;
            return Ok(());
        }
    };

    let players: Vec<&Member> = guild
        .members
        .values()
        .filter(|m| m.roles.contains(&role.id))
        .collect();

    if players.len() != total_signups as usize {
        // Update in db
        sqlx::query!(
            "
            INSERT INTO config (guild_id, total_signups) VALUES ($1, $2)
            ON CONFLICT (guild_id) DO UPDATE SET total_signups = $2;
            ",
            guild.id.0 as i64,
            players.len() as i16
        )
        .execute(pool)
        .await?;
    }

    msg.channel_id
        .say(&ctx.http, "Synced total signups.")
        .await?;

    Ok(())
}

/// Shows total number of sign-ups.
///
/// **Usage:** `[p]total`
///
/// If the total doesn't match the actual total, you can use the `[p]synctotal` command
/// to sync them.
#[command("total")]
async fn total_signups(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = match msg.guild(ctx).await {
        Some(i) => i,
        None => return Err(CommandError::from("Couldn't fetch details of this server.")),
    };

    let data_read = ctx.data.read().await;
    let pool = data_read.get::<ConnectionPool>().unwrap();

    let total = match sqlx::query!(
        "
        SELECT total_signups FROM config WHERE guild_id = $1;
        ",
        guild.id.0 as i64
    )
    .fetch_one(pool)
    .await
    {
        Ok(i) => i.total_signups.unwrap_or(0),
        Err(_) => {
            return Err(CommandError::from(
                "Couldn't fetch details from the database.",
            ))
        }
    };

    msg.channel_id
        .say(
            &ctx.http,
            format!(
                "
            `{}` people are signed up.\
            \n\nIf you think the count is not correct, use the `synctotal` command \
            to fix the count.
            ",
                total
            )
            .trim(),
        )
        .await?;

    Ok(())
}

/// Creates private channels for all members with Player role.
///
/// **Usage:** `[p]playerchats`
///
/// **Alias:** `pc`
///
/// You can supply a name for the category which is created to keep all the channels.
/// "Private Chats" is used by default.
///
/// **Note:** Do not delete the private channels created for users who are Mafia.
/// Anyone can use a custom Discord client to view the complete list of channel names
/// in a server, irrespective of the permissions. If the find that a few people don't have
/// a channel with their name, they will realise that those few people are Mafia.
///
/// **Example**
///
/// *Assuming `Arius` and `Ligi` have Player role.*
///
/// Command: `[p]pc Secret Chats`
///
/// Result: The bot will create a category called "Secret Chats" with 2 channels in it,
/// named `arius` and `ligi`. `arius` will only be visible to the hosts, `Arius` and the bot.
/// Similarly, `ligi` will be visible to the hosts, `Ligi` and the bot.
///
/// The bot asks for confirmation before creating any channels.
#[command("playerchats")]
#[aliases("pc")]
async fn players_chats(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    // Optionally specified category name
    let cat_name = match args.remains() {
        Some(n) => n,
        None => "Private Chats",
    };

    let guild = match msg.guild(ctx).await {
        Some(i) => i,
        None => return Err(CommandError::from("Couldn't fetch details of this server.")),
    };

    let confirm_msg = msg
        .channel_id
        .say(&ctx.http, "Are you sure you want to create player chats?")
        .await?;

    match yes_or_no_pred(&ctx, &msg, &confirm_msg).await {
        Ok(b) if b => (),
        Ok(_) => {
            msg.channel_id
                .say(&ctx.http, "Cancelled player chats creation.")
                .await?;
            return Ok(());
        }
        Err(e) => return Err(e),
    };

    let me = ctx.cache.current_user().await;
    let default_role = RoleId(guild.id.0);

    // Allow hosts and the bot to talk in the category.
    let cat_perms = vec![
        PermissionOverwrite {
            allow: Permissions::empty(),
            deny: Permissions::READ_MESSAGES,
            kind: PermissionOverwriteType::Role(default_role),
        },
        PermissionOverwrite {
            allow: Permissions::READ_MESSAGES | Permissions::SEND_MESSAGES,
            deny: Permissions::empty(),
            kind: PermissionOverwriteType::Member(me.id),
        },
    ];

    let data_read = ctx.data.read().await;
    let pool = data_read.get::<ConnectionPool>().unwrap();

    let res = match sqlx::query!(
        "SELECT host_role_id, player_role_id FROM config WHERE guild_id = $1",
        guild.id.0 as i64
    )
    .fetch_one(pool)
    .await
    {
        Ok(r) => r,
        Err(_) => {
            return Err(CommandError::from(
                "Couldn't fetch details from the database.",
            ))
        }
    };

    let host_role = match get_role(ctx, guild.id, res.host_role_id).await {
        Ok(r) => Some(r),
        Err(_) => None,
    };

    let player_role = match get_role(ctx, guild.id, res.player_role_id).await {
        Ok(r) => r,
        Err(_) => {
            msg.channel_id
                .say(
                    &ctx.http,
                    "Either the Player role is not set, or it got deleted.",
                )
                .await?;
            return Ok(());
        }
    };

    let category = match guild
        .create_channel(ctx, |c| {
            c.name(cat_name)
                .kind(ChannelType::Category)
                .permissions(cat_perms)
        })
        .await
    {
        Ok(r) => r,
        Err(_) => {
            msg.channel_id
                .say(&ctx.http, "Could not create a category.")
                .await?;
            return Ok(());
        }
    };

    let players: Vec<&Member> = guild
        .members
        .values()
        .filter(|m| m.roles.contains(&player_role.id))
        .collect();

    // The following operation may take some time to finish, depending on number of players.
    // So, we make the bot appear as if it's typing something in order to indicate that the process
    // is still going on.
    if let Some(c) = ctx.cache.guild_channel(msg.channel_id).await {
        c.broadcast_typing(ctx).await?
    }

    // Compute this only once.
    let allow_perms = Permissions::READ_MESSAGES
        | Permissions::SEND_MESSAGES
        | Permissions::ADD_REACTIONS
        | Permissions::EMBED_LINKS
        | Permissions::READ_MESSAGE_HISTORY
        | Permissions::ATTACH_FILES;

    for player in players {
        let mut overwrites = vec![
            PermissionOverwrite {
                allow: Permissions::empty(),
                deny: Permissions::READ_MESSAGES,
                kind: PermissionOverwriteType::Role(default_role),
            },
            PermissionOverwrite {
                allow: Permissions::READ_MESSAGES
                    | Permissions::SEND_MESSAGES
                    | Permissions::ADD_REACTIONS,
                deny: Permissions::empty(),
                kind: PermissionOverwriteType::Member(me.id),
            },
            PermissionOverwrite {
                allow: allow_perms,
                deny: Permissions::empty(),
                kind: PermissionOverwriteType::Member(player.user.id),
            },
        ];

        if let Some(r) = &host_role {
            overwrites.push(PermissionOverwrite {
                allow: allow_perms,
                deny: Permissions::empty(),
                kind: PermissionOverwriteType::Role(r.id),
            })
        };

        match guild
            .create_channel(ctx, |c| {
                c.name(&player.user.name)
                    .kind(ChannelType::Text)
                    .permissions(overwrites)
                    .category(&category)
            })
            .await
        {
            Ok(_) => (),
            Err(_) => {
                // We were able to create a category earlier, but we're still
                // checking again to be absolutely sure.
                msg.channel_id
                    .say(
                        &ctx.http,
                        "I couldn't create a channel. Please check my permissions.",
                    )
                    .await?;
                return Ok(());
            }
        };
    }

    msg.channel_id
        .say(&ctx.http, "Created player chats.")
        .await?;
    Ok(())
}

/// Creates a private channel for spectators.
///
/// **Usage:** `[p]specchat`
///
/// **Alias:** `spectatorchat`
///
/// The channel is called `spectator-chat`. Spectator role is required for this
/// command to work.
#[command("specchat")]
#[aliases("spectatorchat")]
async fn spec_chat(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = match msg.guild(ctx).await {
        Some(i) => i,
        None => return Err(CommandError::from("Couldn't fetch details of this server.")),
    };

    // Get spec role.
    let data_read = ctx.data.read().await;
    let pool = data_read.get::<ConnectionPool>().unwrap();

    let role = match sqlx::query!(
        "SELECT spec_role_id FROM config WHERE guild_id = $1",
        guild.id.0 as i64
    )
    .fetch_one(pool)
    .await
    {
        Ok(i) => match get_role(ctx, guild.id, i.spec_role_id).await {
            Ok(r) => r,
            Err(_) => {
                msg.channel_id
                    .say(&ctx.http, "Spectator role doesn't exist.")
                    .await?;
                return Ok(());
            }
        },
        Err(_) => {
            return Err(CommandError::from(
                "Couldn't fetch details from the database.",
            ))
        }
    };

    let perms = vec![
        PermissionOverwrite {
            allow: Permissions::empty(),
            deny: Permissions::READ_MESSAGES,
            kind: PermissionOverwriteType::Role(RoleId(guild.id.0)),
        },
        PermissionOverwrite {
            allow: Permissions::READ_MESSAGES | Permissions::SEND_MESSAGES,
            deny: Permissions::empty(),
            kind: PermissionOverwriteType::Role(role.id),
        },
    ];

    let channel = match guild
        .create_channel(&ctx.http, |c| {
            c.name("spectator-chat")
                .kind(ChannelType::Text)
                .permissions(perms)
        })
        .await
    {
        Ok(c) => c,
        Err(_) => {
            msg.channel_id
                .say(&ctx.http, "I'm unable to create a channel.")
                .await?;
            return Ok(());
        }
    };

    msg.channel_id
        .say(&ctx.http, format!("Created {}.", channel.mention()))
        .await?;

    Ok(())
}

/// Creates a private channel for specified mafia members.
///
/// **Usage:** `[p]mafiachat <mafia_1 [mafia_2 [...]]>`
///
/// **Alias:** `mafchat`
///
/// You must supply the exact name, mention or ID of **each** mafia member
/// to enable them to see the resultant channel.
///
/// The created channel will be called `mafia-chat`.
///
/// **Example**
///
/// *Assuming `Arius#5544`, `Ligi#1241` and `Craw#4421` are mafia.*
/// *Ligi is a unique name in the server. Arius has ID 324967676655173642.*
///
/// Command: `[p]mafchat 324967676655173642 Ligi Craw#4421`
///
/// Result: The bot will create a channel called "mafia-chat". The hosts,
/// `Arius`, `Ligi`, `Craw` and the bot will be able to see the channel.
#[command("mafiachat")]
#[aliases("mafchat")]
#[min_args(1)]
async fn mafia_chat(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild = match msg.guild(ctx).await {
        Some(i) => i,
        None => return Err(CommandError::from("Couldn't fetch details of this server.")),
    };

    // Parse args to get users.
    let mut members = Vec::new();
    for arg in args.iter() {
        // members.push(value: T)
        let parsed_arg = match arg {
            Ok(a) => a,
            Err(_) => continue,
        };
        match get_member(ctx, guild.id, Some(&parsed_arg)).await {
            Ok(m) => members.push(m),
            Err(_) => {
                msg.channel_id
                    .say(&ctx.http, format!("No member found from {}.", &parsed_arg))
                    .await?;
                return Ok(());
            }
        }
    }

    let mut perms = vec![PermissionOverwrite {
        allow: Permissions::empty(),
        deny: Permissions::READ_MESSAGES,
        kind: PermissionOverwriteType::Role(RoleId(guild.id.0)),
    }];

    let allow_perms = Permissions::READ_MESSAGES
        | Permissions::SEND_MESSAGES
        | Permissions::ADD_REACTIONS
        | Permissions::EMBED_LINKS
        | Permissions::READ_MESSAGE_HISTORY
        | Permissions::ATTACH_FILES;

    for member in members {
        perms.push(PermissionOverwrite {
            allow: allow_perms,
            deny: Permissions::empty(),
            kind: PermissionOverwriteType::Member(member.user.id),
        });
    }

    let channel = match guild
        .create_channel(&ctx.http, |c| {
            c.name("mafia-chat")
                .kind(ChannelType::Text)
                .permissions(perms)
        })
        .await
    {
        Ok(c) => c,
        Err(_) => {
            msg.channel_id
                .say(&ctx.http, "I'm unable to create a channel.")
                .await?;
            return Ok(());
        }
    };

    msg.channel_id
        .say(&ctx.http, format!("Created {}.", channel.mention()))
        .await?;

    Ok(())
}

/// Creates a category for a new cycle with day, votes and night channels.
///
/// **Usage:** `[p]cycle [number]`
///
/// If you use the bot to create all cycle channels, the bot will be able to detect
/// the correct cycle number. If you didn't use the bot to create all cycles,
/// please specify the cycle number in the command.
///
/// The category is named "Cycle x". The day, votes and night channels will be
/// called "day-x", "day-x-voting" and "night-x" respectively.
///
/// Day and votes channels will be visible to everyone, and all Players will be able
/// to write in it. Night channel will remain hidden.
///
/// *x is the cycle number*
///
/// The bot asks for confirmation before creating the channels.
#[command("cycle")]
async fn create_cycle(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
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

    // Parse args to check if user supplied a cycle number.
    // If not, use number in database after adding one.
    let number: i16 = match args.message().parse() {
        Ok(n) => n,
        Err(_) => cycle.number + 1,
    };

    // Confirmation for cycle creation.
    let confirm_msg = msg
        .channel_id
        .say(
            &ctx.http,
            format!(
                "
                Are you sure you want to create cycle `{}` channels? Make \
                sure you have the day text ready. Users will be able to talk \
                in the day and vote channels as soon as they are created.
                ",
                number
            )
            .trim(),
        )
        .await?;

    match yes_or_no_pred(&ctx, &msg, &confirm_msg).await {
        Ok(b) if b => (),
        Ok(_) => {
            msg.channel_id
                .say(&ctx.http, "Cancelled cycle creation.")
                .await?;
            return Ok(());
        }
        Err(e) => return Err(e),
    };

    // User confirmed. Let's do it.
    let role = match get_role(ctx, guild.id, data.player_role_id).await {
        Ok(r) => r,
        Err(_) => {
            msg.channel_id
                .say(&ctx.http, "Player role doesn't exist or is invalid now.")
                .await?;
            return Ok(());
        }
    };

    let me = ctx.cache.current_user().await;
    let default_role = RoleId(guild.id.0);

    let perms = vec![
        PermissionOverwrite {
            allow: Permissions::READ_MESSAGES | Permissions::ADD_REACTIONS,
            deny: Permissions::SEND_MESSAGES,
            kind: PermissionOverwriteType::Role(default_role),
        },
        PermissionOverwrite {
            allow: Permissions::SEND_MESSAGES,
            deny: Permissions::ATTACH_FILES,
            kind: PermissionOverwriteType::Role(role.id),
        },
        PermissionOverwrite {
            allow: Permissions::SEND_MESSAGES | Permissions::EMBED_LINKS,
            deny: Permissions::empty(),
            kind: PermissionOverwriteType::Member(me.id),
        },
    ];

    let night_perms = vec![
        PermissionOverwrite {
            allow: Permissions::empty(),
            deny: Permissions::READ_MESSAGES,
            kind: PermissionOverwriteType::Role(RoleId(guild.id.0)),
        },
        PermissionOverwrite {
            allow: Permissions::READ_MESSAGES
                | Permissions::SEND_MESSAGES
                | Permissions::EMBED_LINKS,
            deny: Permissions::empty(),
            kind: PermissionOverwriteType::Member(me.id),
        },
    ];

    let category = match guild
        .create_channel(&ctx.http, |c| {
            c.name(format!("Day {}", number))
                .kind(ChannelType::Category)
                .permissions(perms)
        })
        .await
    {
        Ok(c) => c,
        Err(_) => {
            msg.channel_id
                .say(&ctx.http, "I'm unable to create a category.")
                .await?;
            return Ok(());
        }
    };

    // As the bot was able to create a category, we can assume it's permissions
    // won't change in such short duration.
    let day = guild
        .create_channel(&ctx.http, |c| {
            c.name(format!("day-{}", number))
                .kind(ChannelType::Text)
                .category(&category)
        })
        .await?;

    let votes = guild
        .create_channel(&ctx.http, |c| {
            c.name(format!("day-{}-voting", number))
                .kind(ChannelType::Text)
                .category(&category)
        })
        .await?;

    let night = guild
        .create_channel(&ctx.http, |c| {
            c.name(format!("night-{}", number))
                .kind(ChannelType::Text)
                .category(&category)
                .permissions(night_perms)
        })
        .await?;

    // Time to update the database.
    sqlx::query!(
        r#"
        INSERT INTO config(
            guild_id,
            cycle,
            na_submitted
        ) VALUES (
            $1,
            $2,
            null
        ) ON CONFLICT (guild_id)
        DO UPDATE SET
            cycle = $2,
            na_submitted = null;
        "#,
        guild.id.0 as i64,
        serde_json::to_value(Cycle {
            number: number,
            day: Some(day.id.0 as i64),
            night: Some(night.id.0 as i64),
            votes: Some(votes.id.0 as i64)
        })
        .unwrap()
    )
    .execute(pool)
    .await?;

    msg.channel_id
        .say(
            &ctx.http,
            format!("Created cycle `{}` category and channels!", number),
        )
        .await?;

    Ok(())
}

/// Closes the day channels and opens the night channel.
///
/// **Usage:** `[p]night`
///
/// Day and votes channels will remain visible to everyone but Players won't be
/// able to write in them. Night channel will become visible to everyone, and Players
/// will be able to write in it.
///
/// If the Night Actions channel is set up, the bot will send a message marking the
/// beginning of night x, where x is the cycle number. If Night Actions channel is not
/// set up, the bot will create a new channel called "night-actions" and send the message
/// in it. The channel will be visible by hosts and the bot only.
///
/// The bot asks for confirmation before executing the command.
#[command]
async fn night(ctx: &Context, msg: &Message) -> CommandResult {
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

    let cycle: Cycle = match data.cycle {
        Some(c) => c.0,
        None => {
            return Err(CommandError::from(
                "I couldn't get cycle details in the database.",
            ))
        }
    };

    // Confirmation for cycle creation.
    let confirm_msg = msg
        .channel_id
        .say(
            &ctx.http,
            format!(
                "
                Are you sure you want to start night `{}`? Make \
                sure you have already posted the night-starting text. \
                Users will be able to talk in the night channel as soon \
                as the channel is opened.
                ",
                cycle.number
            )
            .trim(),
        )
        .await?;

    match yes_or_no_pred(&ctx, &msg, &confirm_msg).await {
        Ok(b) if b => (),
        Ok(_) => {
            msg.channel_id
                .say(&ctx.http, "Cancelled starting of night.")
                .await?;
            return Ok(());
        }
        Err(e) => return Err(e),
    };

    let day: GuildChannel = match get_channel_from_id(ctx, guild.id, cycle.day).await {
        Ok(c) => c,
        Err(_) => {
            msg.channel_id
                .say(&ctx.http, "I couldn't get the current day channel.")
                .await?;
            return Ok(());
        }
    };
    let votes: GuildChannel = match get_channel_from_id(ctx, guild.id, cycle.votes).await {
        Ok(c) => c,
        Err(_) => {
            msg.channel_id
                .say(&ctx.http, "I couldn't get the current votes channel.")
                .await?;
            return Ok(());
        }
    };
    let night: GuildChannel = match get_channel_from_id(ctx, guild.id, cycle.night).await {
        Ok(c) => c,
        Err(_) => {
            msg.channel_id
                .say(&ctx.http, "I couldn't get the current night channel.")
                .await?;
            return Ok(());
        }
    };

    let role = match get_role(ctx, guild.id, data.player_role_id).await {
        Ok(r) => r,
        Err(_) => {
            msg.channel_id
                .say(&ctx.http, "Player role doesn't exist or is invalid now.")
                .await?;
            return Ok(());
        }
    };

    // Remove overwrites for `Player` role from day channels.
    match day
        .delete_permission(&ctx.http, PermissionOverwriteType::Role(role.id))
        .await
    {
        Ok(_) => (),
        Err(_) => {
            msg.channel_id
                .say(&ctx.http, "I couldn't change permissions for the channels.")
                .await?;
            return Ok(());
        }
    };
    match votes
        .delete_permission(&ctx.http, PermissionOverwriteType::Role(role.id))
        .await
    {
        Ok(_) => (),
        Err(_) => {
            msg.channel_id
                .say(
                    &ctx.http,
                    "I couldn't change permissions for the night channel.",
                )
                .await?;
            return Ok(());
        }
    };

    // Remove overwrites for everyone from the night channel.
    let overwrites = &night.permission_overwrites;
    for overwrite in overwrites {
        match overwrite.kind {
            PermissionOverwriteType::Member(m) => {
                night
                    .delete_permission(&ctx.http, PermissionOverwriteType::Member(m))
                    .await?;
            }
            PermissionOverwriteType::Role(r) => {
                night
                    .delete_permission(&ctx.http, PermissionOverwriteType::Role(r))
                    .await?;
            }
            _ => (),
        };
    }

    msg.channel_id
        .say(&ctx.http, format!("Night {} channel opened.", cycle.number))
        .await?;

    // We'll handle night actions channel now.
    // Errors with that channel shouldn't affect opening and closing of night/day channels.

    // First, clear list of users who have submitted NA.
    sqlx::query!(
        "
        INSERT INTO config(
            guild_id, na_submitted
        ) VALUES (
            $1, '{}'
        ) ON CONFLICT (guild_id)
        DO UPDATE SET na_submitted = '{}';
        ",
        guild.id.0 as i64
    )
    .execute(pool)
    .await?;

    // Now, fetch the channel or create it, and then send night beginning message.
    let channel = match get_na_channel(&ctx, &guild, &pool).await {
        Ok(c) => c,
        Err(e) => return Err(CommandError::from(e)),
    };

    match channel
        .id
        .say(
            &ctx.http,
            format!("**Night {} begins!**\n\n\n\n\u{200b}", cycle.number),
        )
        .await
    {
        Ok(_) => (),
        Err(_) => {
            msg.channel_id
                .say(
                    &ctx.http,
                    "I couldn't send a message in the night actions channel.",
                )
                .await?;
        }
    };

    Ok(())
}

/// Returns night actions channel if it exists. If it doesn't, it creates
/// a new channel, adds it to the database, and then returns it.
pub(crate) async fn get_na_channel<'a>(
    ctx: &Context,
    guild: &Guild,
    pool: &'a sqlx::Pool<sqlx::postgres::PgConnection>,
) -> Result<GuildChannel, &'static str> {
    let na_channel_id = match sqlx::query!(
        "SELECT na_channel_id FROM config WHERE guild_id = $1;",
        guild.id.0 as i64
    )
    .fetch_one(pool)
    .await
    {
        Ok(r) => r.na_channel_id,
        Err(_) => return Err("Unable to fetch details of night actions channel from database."),
    };

    if let Ok(c) = get_channel_from_id(ctx, guild.id, na_channel_id).await {
        return Ok(c);
    }

    // Time to create the NA channel, add it to the database and then return it.
    let perms = vec![
        PermissionOverwrite {
            allow: Permissions::empty(),
            deny: Permissions::READ_MESSAGES,
            kind: PermissionOverwriteType::Role(RoleId(guild.id.0)),
        },
        PermissionOverwrite {
            allow: Permissions::READ_MESSAGES
                | Permissions::SEND_MESSAGES
                | Permissions::ADD_REACTIONS,
            deny: Permissions::empty(),
            kind: PermissionOverwriteType::Member(ctx.cache.current_user_id().await),
        },
    ];

    let channel = match guild
        .create_channel(&ctx.http, |c| c.name("night-actions").permissions(perms))
        .await
    {
        Ok(c) => c,
        Err(_) => return Err("Unable to create a channel for night actions."),
    };

    match sqlx::query!(
        "
        INSERT INTO config (
            guild_id, na_channel_id
        )
        VALUES (
            $1, $2
        ) ON CONFLICT (guild_id)
        DO UPDATE SET na_channel_id = $2;
        ",
        guild.id.0 as i64,
        channel.id.0 as i64
    )
    .execute(pool)
    .await
    {
        Ok(_) => (),
        Err(_) => return Err("Unable to add newly created channel to database."),
    };

    Ok(channel)
}

/// Kills a player by removing player role and adding dead player role.
///
/// **Usage:** `[p]kill <user>`
///
/// The command fails if player and dead player roles are not set up.
#[command("kill")]
#[min_args(1)]
async fn kill_player(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let guild = match msg.guild(ctx).await {
        Some(i) => i,
        None => return Err(CommandError::from("Couldn't fetch details of this server.")),
    };

    let input = args.message().to_string();
    // Get tagged member.
    let mut member = match get_member(ctx, guild.id, Some(&input)).await {
        Ok(m) => m,
        Err(_) => {
            msg.channel_id
                .say(&ctx.http, format!("No member found from `{}`.", input))
                .await?;
            return Ok(());
        }
    };

    let data_read = ctx.data.read().await;
    let pool = data_read.get::<ConnectionPool>().unwrap();

    let res = match sqlx::query!(
        "SELECT dead_role_id, player_role_id FROM config WHERE guild_id = $1;",
        guild.id.0 as i64
    )
    .fetch_one(pool)
    .await
    {
        Ok(r) => r,
        Err(_) => {
            return Err(CommandError::from(
                "Unable to fetch details of set roles from database.",
            ))
        }
    };

    let player_role = match get_role(&ctx, guild.id, res.player_role_id).await {
        Ok(r) => r,
        Err(_) => {
            msg.channel_id
                .say(&ctx.http, "I couldn't find the player role.")
                .await?;
            return Ok(());
        }
    };

    let dead_role = match get_role(&ctx, guild.id, res.dead_role_id).await {
        Ok(r) => r,
        Err(_) => {
            msg.channel_id
                .say(&ctx.http, "I couldn't find the dead player role.")
                .await?;
            return Ok(());
        }
    };

    if !member.roles.contains(&player_role.id) {
        msg.channel_id
            .say(&ctx.http, "User doesn't have the player role!")
            .await?;
        return Ok(());
    }

    // Remove player role.
    if member.remove_role(&ctx.http, player_role.id).await.is_err() {
        msg.channel_id
            .say(&ctx.http, "I couldn't remove player role from the user.")
            .await?;
        return Ok(());
    }

    // Add dead player role.
    if member.add_role(&ctx.http, dead_role.id).await.is_err() {
        msg.channel_id
            .say(
                &ctx.http,
                "I couldn't add the dead player role to the user.",
            )
            .await?;
        return Ok(());
    }

    msg.channel_id
        .say(
            &ctx.http,
            "Removed player role and added dead player role to the user!",
        )
        .await?;

    Ok(())
}

/// Generates player list and sends it in the specified channel.
///
/// **Usage:** `[p]playerlist <channel>`
///
/// **Aliases:** `plist`, `pl`
///
/// The player role must be set for this command to work.
///
/// I need the permission to embed links in the specified channel.
#[command("playerlist")]
#[aliases("plist", "pl")]
#[min_args(1)]
async fn player_list(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let guild = match msg.guild(ctx).await {
        Some(i) => i,
        None => return Err(CommandError::from("Couldn't fetch details of this server.")),
    };

    // Get argument channel.
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

    let mut players_str = String::new();
    for (i, player) in players.iter().enumerate() {
        let _ = write!(players_str, "\n{}. {}", i + 1, player);
    }

    let sent = channel
        .send_message(&ctx.http, |m| {
            m.embed(|e| {
                e.title("Player List");
                e.description(players_str);
                e.colour(role.colour);
                e.footer(|f| {
                    f.text(format!("Total Players: {0}", players.len()));

                    f
                });

                e
            });

            m
        })
        .await;

    if sent.is_err() {
        msg.channel_id.say(
            &ctx.http,
            format!(
                "I couldn't send the player list. Please check if I have permissions to embed link in {}.",
                channel.mention()
            )
        ).await?;
    }

    Ok(())
}

#[group("Host Utility")]
#[description = "Utility commands for hosts."]
#[only_in("guilds")]
#[checks("is_host_or_admin")]
#[commands(
    randomize_roles,
    sync_total,
    total_signups,
    players_chats,
    spec_chat,
    mafia_chat,
    create_cycle,
    night,
    kill_player,
    player_list
)]
struct Utilities;
