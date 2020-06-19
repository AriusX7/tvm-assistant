// Checks to run before command invocation are defined here.

use crate::ConnectionPool;
use serenity::{
    framework::standard::{macros::check, CheckResult},
    model::channel::Message,
    prelude::Context,
};

#[check]
#[name = "is_host_or_admin"]
// Check to restrict a command to server admins and game hosts.
pub(crate) async fn is_host_or_admin_check(ctx: &Context, msg: &Message) -> CheckResult {
    let guild = match msg.guild(&ctx).await {
        Some(i) => i,
        None => return CheckResult::new_user("Command cannot be used in DMs."),
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
                    return CheckResult::Success;
                }
            }
        }
    };

    if let Some(m) = guild.members.get(&msg.author.id) {
        if let Ok(p) = m.permissions(&ctx).await {
            if p.administrator() {
                return CheckResult::Success;
            }
        }
    };

    CheckResult::new_user("You don't have enough permissions to run this command.")
}

#[check]
#[name = "tvmset_lock"]
// Check to restrict command usage when TvM settings are locked.
pub(crate) async fn tvmset_lock_check(ctx: &Context, msg: &Message) -> CheckResult {
    let data_read = ctx.data.read().await;
    let pool = data_read.get::<ConnectionPool>().unwrap();

    let guild_id = match msg.guild_id {
        Some(i) => i.0,
        None => return CheckResult::new_user("Command cannot be used in DMs."),
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
        CheckResult::Success
    } else {
        CheckResult::new_user("TvM settings are locked!")
    }
}
