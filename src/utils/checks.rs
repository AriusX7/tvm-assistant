// Checks to run before command invocation are defined here.

use crate::ConnectionPool;
use serenity::{
    framework::standard::{macros::check, Reason},
    model::channel::Message,
    prelude::Context,
};

// Check to restrict a command to server admins and game hosts.
#[check]
#[name = "is_host_or_admin"]
pub(crate) async fn is_host_or_admin_check(ctx: &Context, msg: &Message) -> Result<(), Reason> {
    let guild = match msg.guild(&ctx).await {
        Some(i) => i,
        None => return Err(Reason::User("Command cannot be used in DMs.".to_string())),
    };

    if let Some(m) = guild.members.get(&msg.author.id) {
        if let Ok(p) = m.permissions(&ctx).await {
            if p.administrator() {
                return Ok(());
            }
        }
    };

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
            if let Ok(res) = msg.author.has_role(&ctx, guild.id, i as u64).await {
                if res {
                    return Ok(());
                }
            }
        }
    };

    Err(Reason::User(
        "You don't have enough permissions to run this command.".to_string(),
    ))
}

// Check to restrict command usage when TvM settings are locked.
#[check]
#[name = "tvmset_lock"]
pub(crate) async fn tvmset_lock_check(ctx: &Context, msg: &Message) -> Result<(), Reason> {
    let data_read = ctx.data.read().await;
    let pool = data_read.get::<ConnectionPool>().unwrap();

    let guild_id = match msg.guild_id {
        Some(i) => i.0,
        None => return Err(Reason::User("Command cannot be used in DMs.".to_string())),
    };

    let setting = match sqlx::query!(
        "SELECT tvmset_lock FROM config WHERE guild_id = $1",
        guild_id as i64
    )
    .fetch_one(pool)
    .await
    {
        Ok(r) => {
            match r.tvmset_lock {
                Some(i) => i,
                // Not set yet for whatever reason, false by default.
                None => false,
            }
        }
        // Not set yet for whatever reason, false by default.
        Err(_) => false,
    };

    if !setting {
        Ok(())
    } else {
        Err(Reason::User("TvM settings are locked!".to_string()))
    }
}
