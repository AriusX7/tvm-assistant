mod commands;
mod events;
mod utils;

use commands::{help::help_command, host::*, logging::*, meta::*, owner::*, setup::*, user::*};
use events::{message_delete_bulk_handler, message_delete_handler, message_update_handler};
use log::{error, info};
use serenity::{
    async_trait,
    client::bridge::gateway::{GatewayIntents, ShardManager},
    framework::{
        standard::{
            macros::{help, hook},
            Args, CommandGroup, CommandResult, DispatchError, HelpOptions, Reason,
        },
        StandardFramework,
    },
    http::Http,
    model::{
        event::ResumedEvent,
        gateway::Ready,
        prelude::{ChannelId, Guild, Message, MessageId, MessageUpdateEvent, UserId},
    },
    prelude::*,
};
use sqlx::PgPool;
use std::{collections::HashSet, env, sync::Arc};
use utils::database::{obtain_pool, initialize_tables};

#[macro_use]
extern crate lazy_static;

struct ShardManagerContainer;
struct ConnectionPool;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

impl TypeMapKey for ConnectionPool {
    type Value = PgPool;
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        info!("Connected as {}", ready.user.name);
    }

    async fn resume(&self, _: Context, _: ResumedEvent) {
        info!("Resumed");
    }

    async fn guild_create(&self, ctx: Context, guild: Guild, is_new: bool) {
        // We'll initialize the database tables for a guild if it's new.

        if !is_new {
            return;
        }

        initialize_tables(&ctx, &guild).await;
    }

    async fn message_update(
        &self,
        ctx: Context,
        old_if_available: Option<Message>,
        new: Option<Message>,
        event: MessageUpdateEvent,
    ) {
        message_update_handler(ctx, old_if_available, new, event).await;
    }

    async fn message_delete(
        &self,
        ctx: Context,
        channel_id: ChannelId,
        deleted_message_id: MessageId,
    ) {
        message_delete_handler(ctx, channel_id, deleted_message_id).await;
    }

    async fn message_delete_bulk(
        &self,
        ctx: Context,
        channel_id: ChannelId,
        multiple_deleted_messages_ids: Vec<MessageId>,
    ) {
        message_delete_bulk_handler(ctx, channel_id, multiple_deleted_messages_ids).await;
    }
}

#[help]
async fn my_help(
    ctx: &'static Context,
    msg: &'static Message,
    args: Args,
    help_options: &HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    help_command(&ctx, &msg, args, help_options, groups, owners).await
}

// This is for errors that happen before command execution.
#[hook]
async fn on_dispatch_error(ctx: &Context, msg: &Message, error: DispatchError) {
    match error {
        DispatchError::NotEnoughArguments { min, given } => {
            let s = {
                if given == 0 && min == 1 {
                    "I need an argument to run this command.".to_string()
                } else if given == 0 {
                    format!("I need at least {} arguments to run this command.", min)
                } else {
                    format!(
                        "I need {} arguments to run this command, but I was only given {}.",
                        min, given
                    )
                }
            };
            let _ = msg.channel_id.say(ctx, s).await;
        }
        DispatchError::OnlyForGuilds => {
            let _ = msg
                .channel_id
                .say(ctx, "This command can only be used in a server.")
                .await;
        }
        DispatchError::OnlyForDM => {
            let _ = msg
                .channel_id
                .say(ctx, "This command can only be used in DMs.")
                .await;
        }
        DispatchError::IgnoredBot {} => {
            return;
        }
        DispatchError::CheckFailed(_, reason) => {
            if let Reason::User(r) = reason {
                let _ = msg.channel_id.say(ctx, r).await;
            }
        }
        _ => {
            error!("Unhandled dispatch error: {:?}", error);
        }
    }
}

// This function executes every time a command finishes executing.
// It's used here to handle errors that happen in the middle of the command.
#[hook]
async fn after(ctx: &Context, msg: &Message, cmd_name: &str, error: CommandResult) {
    if let Err(why) = &error {
        error!("Error while running command {}", &cmd_name);
        error!("{:?}", &error);

        let err = why.0.to_string();
        if msg.channel_id.say(ctx, &err).await.is_err() {
            error!(
                "Unable to send messages on channel id {}",
                &msg.channel_id.0
            );
        };
    }
}

#[hook]
// Sets a custom prefix for a guild.
pub async fn dynamic_prefix(ctx: &Context, msg: &Message) -> Option<String> {
    let guild_id = &msg.guild_id;

    let prefix: String;
    if let Some(id) = guild_id {
        let data = ctx.data.read().await;
        let pool = data.get::<ConnectionPool>().unwrap();

        let res = sqlx::query!(
            "SELECT prefix FROM prefixes WHERE guild_id = $1",
            id.0 as i64
        )
        .fetch_one(pool)
        .await;

        prefix = if let Ok(data) = res {
            if let Some(p) = data.prefix {
                p
            } else {
                "-".to_string()
            }
        } else {
            error!("I couldn't query the database for getting guild prefix.");
            "-".to_string()
        }
    } else {
        prefix = "-".to_string();
    };

    Some(prefix)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // This will load the environment variables located at `./.env`, relative to
    // the CWD. See `./.env.example` for an example on how to structure this.
    kankyo::load(false).expect("Failed to load .env file");

    // Initialize the logger to use environment variables.
    env_logger::init();

    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment.");

    let database_url = env::var("DATABASE_URL").expect("Expected database url in the environment.");

    let http = Http::new_with_token(&token);

    // We will fetch your bot's owners and id
    let (owners, bot_id) = match http.get_current_application_info().await {
        Ok(info) => {
            let mut owners = HashSet::new();
            owners.insert(info.owner.id);

            (owners, info.id)
        }
        Err(why) => panic!("Could not access application info: {:?}", why),
    };

    // Create the framework
    let framework = StandardFramework::new()
        .configure(|c| {
            c.owners(owners)
                .dynamic_prefix(dynamic_prefix)
                .with_whitespace(true)
                .on_mention(Some(bot_id))
        })
        .on_dispatch_error(on_dispatch_error)
        .after(after)
        .group(&USERCOMMANDS_GROUP)
        .group(&UTILITIES_GROUP)
        .group(&TVMSET_GROUP)
        .group(&LOGGING_GROUP)
        .group(&MISC_GROUP)
        .group(&OWNER_GROUP)
        .help(&MY_HELP);

    let mut client = Client::new(&token)
        .framework(framework)
        .event_handler(Handler)
        .add_intent({
            let mut intents = GatewayIntents::all();
            intents.remove(GatewayIntents::DIRECT_MESSAGE_TYPING);
            intents.remove(GatewayIntents::GUILD_MESSAGE_TYPING);

            intents
        })
        .await
        .expect("Err creating client");

    // Store 10 messages in cache.
    client.cache_and_http.cache.set_max_messages(10).await;

    {
        let mut data = client.data.write().await;
        data.insert::<ShardManagerContainer>(Arc::clone(&client.shard_manager));

        // Add the database connection to the data.
        let pool = obtain_pool(&database_url).await?;
        data.insert::<ConnectionPool>(pool);
    }

    if let Err(why) = client.start().await {
        error!("Client error: {:?}", why);
    }

    Ok(())
}
