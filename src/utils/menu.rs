// Functionality to send a reaction-based menu. It is currently very bare-bones, with
// only three reactions and very few customisation options.
//
// The functionality has been ported from [`Red-DiscordBot`]'s [`menu`] function.
//
// [`Red-DiscordBot`]: https://github.com/Cog-Creators/Red-DiscordBot/
// [`menu`]: https://github.com/Cog-Creators/Red-DiscordBot/blob/V3/develop/redbot/core/utils/menus.py#L18

use super::embed::Embed;
use futures::stream::StreamExt;
use serenity::{
    builder::CreateEmbed,
    collector::ReactionAction,
    framework::standard::CommandError,
    model::prelude::{Message, Reaction, ReactionType},
    prelude::Context,
};
use std::time::Duration;

lazy_static! {
    static ref EMOJIS: Vec<ReactionType> = vec!['◀'.into(), '❌'.into(), '▶'.into()];
}

#[derive(Debug, Clone)]
pub struct MenuError(pub String);

impl From<serenity::Error> for MenuError {
    fn from(error: serenity::Error) -> Self {
        Self(format!("{}", error))
    }
}

impl From<CommandError> for MenuError {
    fn from(error: CommandError) -> Self {
        Self(error.to_string())
    }
}

pub type MenuResult = std::result::Result<(), MenuError>;

#[derive(Debug, Clone)]
pub struct MenuOptions {
    pub(crate) page: usize,
    pub(crate) timeout: f64,
    pub(crate) message: Option<Message>,
}

impl MenuOptions {
    pub fn new(page: usize, timeout: f64, message: Option<Message>) -> Self {
        Self {
            page,
            timeout,
            message,
        }
    }
}

impl Default for MenuOptions {
    fn default() -> Self {
        Self {
            page: 0,
            timeout: 30.0,
            message: None,
        }
    }
}

pub async fn run(
    ctx: &Context,
    msg: &Message,
    pages: &[CreateEmbed],
    options: &mut MenuOptions,
) -> Result<(usize, Reaction), MenuError> {
    if pages.is_empty() {
        return Err(MenuError(String::from("`pages` is empty.")));
    }

    if options.page > pages.len() - 1 {
        return Err(MenuError(String::from("`page` is out of bounds.")));
    }

    let current_page = &pages[options.page];

    match &options.message {
        Some(message) => {
            message
                .clone()
                .edit(&ctx.http, |m| {
                    m.embed(|e| {
                        e.clone_from(current_page);

                        e
                    });

                    m
                })
                .await?;
        }
        None => {
            let sent_msg = msg
                .channel_id
                .send_message(&ctx.http, |m| {
                    m.embed(|e| {
                        e.clone_from(current_page);

                        e
                    });

                    m
                })
                .await?;

            options.message = Some(sent_msg.clone());
            add_reactions(&ctx, &sent_msg, EMOJIS.as_slice()).await?;
        }
    }

    // options.message is sure to be `Some()` now.
    let message = &options.clone().message.unwrap();
    let mut reaction_collector = message
        .await_reactions(&ctx)
        .timeout(Duration::from_secs_f64(options.timeout))
        .author_id(msg.author.id.0)
        .await;

    let (choice, reaction) = {
        let mut choice = None;
        let mut reaction = None;

        while let Some(item) = reaction_collector.next().await {
            if let ReactionAction::Added(r) = item.as_ref() {
                let r = r.as_ref().clone();
                if let Some(i) = process_reaction(&r, EMOJIS.as_slice()) {
                    choice = Some(i);
                    reaction = Some(r);
                    break;
                }
            }
        }
        (choice, reaction)
    };

    match choice {
        Some(c) => Ok((c, reaction.unwrap())),
        None => Err(MenuError("Quit!".to_string())),
    }
}

async fn add_reactions(ctx: &Context, msg: &Message, emojis: &[ReactionType]) -> MenuResult {
    for emoji in emojis {
        msg.react(&ctx.http, emoji.clone()).await?;
    }

    Ok(())
}

fn process_reaction(reaction: &Reaction, emojis: &[ReactionType]) -> Option<usize> {
    let emoji = &reaction.emoji;
    if emojis.contains(emoji) {
        emojis.iter().position(|e| e == emoji)
    } else {
        None
    }
}

async fn next_page(
    ctx: &Context,
    pages: &[CreateEmbed],
    options: MenuOptions,
    reaction: Reaction,
) -> MenuOptions {
    // Tries to remove the reaction.
    let _ = &reaction.delete(&ctx.http).await;

    if options.page == pages.len() - 1 {
        MenuOptions::new(0, options.timeout, options.message)
    } else {
        MenuOptions::new(options.page + 1, options.timeout, options.message)
    }
}

async fn prev_page(
    ctx: &Context,
    pages: &[CreateEmbed],
    options: MenuOptions,
    reaction: Reaction,
) -> MenuOptions {
    // Tries to remove the reaction.
    let _ = reaction.delete(&ctx.http).await;

    if options.page == 0 {
        MenuOptions::new(pages.len() - 1, options.timeout, options.message)
    } else {
        MenuOptions::new(options.page - 1, options.timeout, options.message)
    }
}

async fn close_menu(
    ctx: &Context,
    _pages: &[CreateEmbed],
    options: MenuOptions,
    _reaction: Reaction,
) -> MenuOptions {
    let _ = options.message.unwrap().delete(&ctx.http).await;

    MenuOptions::default()
}

pub async fn menu(
    ctx: &Context,
    msg: &Message,
    pages: &[Embed],
    options: MenuOptions,
) -> MenuResult {
    // Convert `Embed`s to `CreateEmbed`s.
    let pages: Vec<CreateEmbed> = pages.iter().map(|e| e.clone().get_create_embed()).collect();
    let pages = pages.as_slice();

    let mut options = options;

    loop {
        match run(ctx, msg, pages, &mut options).await {
            Ok((choice, reaction)) => match choice as u8 {
                0 => {
                    options = prev_page(ctx, pages, options, reaction).await;
                }
                1 => {
                    close_menu(ctx, pages, options, reaction).await;
                    break;
                }
                2 => {
                    options = next_page(ctx, pages, options, reaction).await;
                }
                _ => break,
            },
            Err(_) => {
                // Unwrapping here would be fine.
                let _ = options.message.unwrap().delete_reactions(&ctx.http).await;
                break;
            }
        }
    }

    Ok(())
}
