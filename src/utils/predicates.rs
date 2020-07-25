// Predicates to check user's response.
//
// Currently, only a yes-no reaction menu is implemented.

use serenity::{
    collector::ReactionAction, framework::standard::CommandError, model::prelude::*, prelude::*,
};
use std::{sync::Arc, time::Duration};

pub async fn yes_or_no_pred(
    ctx: &Context,
    msg: &Message,
    confirm_msg: &Message,
) -> Result<bool, CommandError> {
    match &confirm_msg.react(&ctx.http, '✅').await {
        Ok(_) => (),
        Err(_) => {
            return Err(CommandError::from(
                "I cannot add reactions to the message.",
            ))
        }
    };
    match &confirm_msg.react(&ctx.http, '❌').await {
        Ok(_) => (),
        Err(_) => {
            return Err(CommandError::from(
                "I cannot add reactions to the message.",
            ))
        }
    };

    let collected_reaction = match confirm_msg
        .await_reaction(&ctx)
        .timeout(Duration::from_secs(30))
        .author_id(msg.author.id.0)
        // .filter(yes_or_no_reaction_filter)
        .await
    {
        Some(r) => r,
        None => {
            return Err(CommandError::from(
                "There was an error processing reactions.",
            ))
        }
    };

    match Arc::try_unwrap(collected_reaction) {
        Ok(r) => match r {
            ReactionAction::Added(x) => Ok(yes_or_no_reaction_filter(&x)),
            ReactionAction::Removed(_) => Ok(false),
        },
        Err(_) => Ok(false),
    }
}

pub fn yes_or_no_reaction_filter(reaction: &Arc<Reaction>) -> bool {
    match &reaction.as_ref().emoji {
        ReactionType::Unicode(e) if e == &String::from("✅") => true,
        _ => false,
    }
}
