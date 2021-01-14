//! Custom help for the bot. The functions defined here are very hacky,
//! but sufficient for this bot.
//!
//! A lot of functionality has been borrowed from [`Red-DiscordBot`]'s help command.
//!
//! [`Red-DiscordBot`]: https://github.com/Cog-Creators/Red-DiscordBot/

use crate::dynamic_prefix;
use crate::utils::constants::{COMMANDS_REFERENCE, EMBED_COLOUR, QUICKSTART};
use serenity::{
    builder::CreateMessage,
    framework::standard::{
        help_commands::has_all_requirements, Args, Command, CommandGroup, CommandResult,
        HelpOptions,
    },
    model::prelude::{Message, UserId},
    prelude::Context,
};
use serenity_utils::{
    builder::embed::{EmbedBuilder, EmbedFieldBuilder},
    formatting::{pagify, PagifyOptions},
    menu::{Menu, MenuOptions},
};
use std::{collections::HashSet, fmt::Write};

pub async fn help_command(
    ctx: &Context,
    msg: &Message,
    args: Args,
    _help_options: &HelpOptions,
    groups: &[&'static CommandGroup],
    _owners: HashSet<UserId>,
) -> CommandResult {
    // get prefix for the guild.
    let prefix = match dynamic_prefix(&ctx, &msg.guild_id).await {
        Some(p) => p,
        None => String::from("-"),
    };

    if args.is_empty() {
        send_bot_help(ctx, msg, groups, prefix).await?;
    } else {
        let name = args.message();

        if let Some(g) = groups
            .iter()
            .find(|g| g.name == name || g.options.prefixes.contains(&name))
        {
            send_group_help(ctx, msg, *g, &prefix).await?;
            return Ok(());
        }

        if let Some(c) = fetch_command(ctx, msg, &name, &groups).await {
            send_command_help(ctx, msg, c.0, c.1, name, &prefix).await?;
            return Ok(());
        }
    }

    Ok(())
}

async fn get_group_text(
    ctx: &Context,
    msg: &Message,
    group: &CommandGroup,
    main_prefix: &str,
) -> String {
    let mut prefix = main_prefix.to_string();

    let group_prefixes = group.options.prefixes;
    if !group_prefixes.is_empty() {
        write!(prefix, "{} ", group_prefixes[0]).unwrap_or(());
    }

    let bot_name = ctx.cache.current_user().await.name;

    let mut group_str = String::new();
    for command in group.options.commands {
        let options = command.options;
        let name = options.names[0];

        if !command_check(ctx, msg, *command, group).await {
            continue;
        }

        let desc = format_description(options.desc, true, &bot_name, main_prefix);
        write!(group_str, "\n**{}{}** {}", prefix, name, desc).unwrap_or(());
    }

    group_str
}

fn format_description(
    description: Option<&str>,
    about_none: bool,
    bot_name: &str,
    prefix: &str,
) -> String {
    match description {
        Some(d) => {
            if let Some(x) = d.lines().next() {
                let x = parse_text(x, prefix, bot_name);
                if x.len() > 70 {
                    x[..67].to_string() + "..."
                } else {
                    x
                }
            } else {
                String::from("No description.")
            }
        }
        None if about_none => String::from("No description."),
        None => String::new(),
    }
}

async fn send_bot_help(
    ctx: &Context,
    msg: &Message,
    groups: &[&'static CommandGroup],
    prefix: String,
) -> CommandResult {
    let user = &ctx.cache.current_user().await;
    let bot_name = &user.name;

    let mut group_fields = Vec::new();
    for group in groups {
        let mut title = format!("**{}**", group.name);
        if group.options.description.is_some() {
            title.push_str(&format!(
                " - {}",
                format_description(group.options.description, true, bot_name, &prefix)
            ));
        }

        let group_text = get_group_text(ctx, msg, *group, &prefix).await;
        if group_text.is_empty() {
            continue;
        }
        let mut pagify_options = PagifyOptions::new();
        pagify_options.page_length(1000).shorten_by(0);
        let pages = pagify(group_text, pagify_options);
        for (i, page) in pages.iter().enumerate() {
            if i >= 1 {
                let _ = write!(title, " **(continued)**");
            }

            group_fields.push((title.clone(), page.clone(), false))
        }
    }

    let mut embed = EmbedBuilder::new();
    embed
        .set_description(format!(
            "Please visit [this page]({}) for full list of commands.\
                \nSet up the bot for your server by following this [quickstart guide]({}).",
            COMMANDS_REFERENCE, QUICKSTART,
        ))
        .add_fields(group_fields)
        .set_footer_with(|f| f.set_text(get_footer(&prefix)));

    make_and_send_embeds(ctx, msg, &embed).await
}

#[allow(clippy::needless_lifetimes)]
async fn fetch_command<'a>(
    ctx: &Context,
    msg: &Message,
    name: &str,
    groups: &'a [&CommandGroup],
) -> Option<(&'a Command, &'a CommandGroup)> {
    for group in groups {
        let prefixes = group.options.prefixes;
        match group.options.commands.iter().find(|c| {
            if c.options.names.contains(&name) && prefixes.is_empty() {
                return true;
            } else {
                for prefix in prefixes {
                    if c.options
                        .names
                        .iter()
                        .any(|n| format!("{} {}", *prefix, n) == name)
                    {
                        return true;
                    }
                }
            }
            false
        }) {
            Some(c) => {
                if !command_check(ctx, msg, *c, group).await {
                    return None;
                }

                return Some((*c, group));
            }
            None => continue,
        };
    }

    None
}

async fn send_group_help(
    ctx: &Context,
    msg: &Message,
    group: &CommandGroup,
    prefix: &str,
) -> CommandResult {
    if !group.options.help_available {
        msg.channel_id.say(&ctx.http, "No help found.").await?;
        return Ok(());
    }

    let user = &ctx.cache.current_user().await;
    let bot_name = &user.name;
    let mut field_value = get_group_text(ctx, msg, group, prefix).await;

    if field_value.is_empty() {
        field_value = String::from("You cannot use any commands.");
    }

    let mut embed = EmbedBuilder::new();
    embed
        .set_description(format_description(
            group.options.description,
            true,
            bot_name,
            prefix,
        ))
        .set_footer_with(|f| f.set_text(get_footer(prefix)))
        .add_field((format!("**{}**", group.name), field_value, false));

    make_and_send_embeds(ctx, msg, &embed).await
}

async fn send_command_help(
    ctx: &Context,
    msg: &Message,
    command: &Command,
    group: &CommandGroup,
    name: &str,
    main_prefix: &str,
) -> CommandResult {
    if !command.options.help_available {
        msg.channel_id.say(&ctx.http, "No help found.").await?;
        return Ok(());
    }

    let user = &ctx.cache.current_user().await;

    let prefixes = group.options.prefixes;
    let prefix = if !prefixes.is_empty() {
        // Get the first prefix.
        format!("{}{} ", main_prefix, prefixes[0])
    } else {
        main_prefix.to_string()
    };

    let desc = parse_text(
        command.options.desc.unwrap_or("No description."),
        &prefix,
        &user.name,
    );

    let mut embed = EmbedBuilder::new();
    embed
        .set_title(format!("Command: {}", name))
        .set_description(desc)
        .set_footer_with(|f| f.set_text(get_footer(main_prefix)));

    make_and_send_embeds(ctx, msg, &embed).await
}

fn parse_text(text: &str, prefix: &str, bot_name: &str) -> String {
    let text = text.replace("[p]", prefix);
    text.replace("[botname]", bot_name)
}

fn get_footer(prefix: &str) -> String {
    format!(
        "Type {0}help <command> for more info on a command. \
        You can also type {0}help <category> for more info on a category.",
        prefix
    )
}

fn group_embed_fields(
    fields: &[EmbedFieldBuilder],
    max_chars: usize,
) -> Vec<Vec<&EmbedFieldBuilder>> {
    let mut current_group = Vec::new();
    let mut ret = Vec::new();

    let mut current_count = 0;

    for (i, f) in fields.iter().enumerate() {
        let f_len = f.name.len() + f.value.len();

        if current_count == 0 || current_count < max_chars || i < 2 {
            current_count += f_len;
            current_group.push(f)
        } else if !current_group.is_empty() {
            ret.push(current_group);
            current_count = f_len;
            current_group = vec![f];
        }
    }
    if !current_group.is_empty() {
        ret.push(current_group);
    }

    ret
}

async fn make_and_send_embeds(ctx: &Context, msg: &Message, embed: &EmbedBuilder) -> CommandResult {
    let mut pages = Vec::new();

    let mut page_char_limit = 750;
    let user = &ctx.cache.current_user().await;

    let author_name = format!("{} Help", user.name);

    // Offset calculation for total embed size.
    // 20 accounts for `*Page {i} of {page_count}*`
    let mut offset = author_name.len() + 20;

    if let Some(footer) = &embed.footer {
        offset += footer.text.len();
    }

    offset += embed
        .description
        .as_ref()
        .map(|d| d.len())
        .unwrap_or_default();
    offset += embed.title.as_ref().map(|t| t.len()).unwrap_or_default();

    if page_char_limit + offset > 5500 {
        page_char_limit = 5500 - offset;
    } else if page_char_limit < 250 {
        page_char_limit = 250;
    }

    let field_groups = group_embed_fields(embed.fields.as_slice(), page_char_limit);
    let total_pages = field_groups.len();

    if field_groups.is_empty() {
        let mut embed = embed.clone();
        // `embed` may have fields already set in. We need to clear them.
        embed.fields.clear();

        embed
            .set_colour(EMBED_COLOUR)
            .set_author_with(|a| a.set_name(&author_name).set_icon_url(&user.face()));

        let mut page = CreateMessage::default();
        page.set_embed(embed.to_create_embed());
        pages.push(page);
    }

    for (i, group) in field_groups.iter().enumerate() {
        let mut embed = embed.clone();

        // `embed` may have fields already set in. We need to clear them.
        embed.fields.clear();

        embed
            .set_colour(EMBED_COLOUR)
            .set_author_with(|a| a.set_name(&author_name).set_icon_url(&user.face()));

        // Just some weird adjustment.
        let mut prev_field: Option<&EmbedFieldBuilder> = None;
        for (j, &field) in group.iter().enumerate() {
            match prev_field {
                Some(prev) => {
                    let prev_name = prev.name.replace("**", "");
                    let name = field.name.replace("**", "");
                    let merged_value = format!("{}{}", prev.name, name);
                    let merge = merged_value.len() <= 1024;
                    if name.contains(&prev_name) && merge {
                        embed.set_field_at(
                            j - 1,
                            EmbedFieldBuilder::new(
                                format!("**{}**", prev_name),
                                merged_value,
                                field.inline,
                            ),
                        );
                    } else {
                        embed.add_field((&field.name, &field.value, field.inline));
                    }
                }
                None => {
                    embed.add_field((&field.name, &field.value, field.inline));
                }
            }
            prev_field = Some(field);
        }

        if total_pages > 1 {
            if let Some(footer) = &mut embed.footer {
                footer.set_text(format!(
                    "Page {} of {} | {}",
                    i + 1,
                    total_pages,
                    &footer.text
                ));
            }
        }

        let mut page = CreateMessage::default();
        page.set_embed(embed.to_create_embed());
        pages.push(page);
    }

    let menu = Menu::new(ctx, msg, pages.as_slice(), MenuOptions::default());

    menu.run().await?;

    Ok(())
}

async fn command_check(
    ctx: &Context,
    msg: &Message,
    command: &Command,
    group: &CommandGroup,
) -> bool {
    // Owner check.
    // We don't want to display any owner command.
    if command.options.owners_only || group.options.owners_only {
        return false;
    }

    if !has_all_requirements(&ctx, command.options, msg).await {
        return false;
    }

    for check in command.options.checks {
        if check.check_in_help {
            let mut args = Args::new("", &[]);

            if (check.function)(ctx, msg, &mut args, command.options)
                .await
                .is_err()
            {
                return false;
            }
        }
    }
    for check in group.options.checks {
        if check.check_in_help {
            let mut args = Args::new("", &[]);

            if (check.function)(ctx, msg, &mut args, command.options)
                .await
                .is_err()
            {
                return false;
            }
        }
    }

    true
}
