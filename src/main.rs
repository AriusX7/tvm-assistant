mod commands;
mod events;
mod utils;

use commands::{help::help_command, host::*, logging::*, meta::*, owner::*, setup::*, user::*};
use dotenv::dotenv;
use events::{message_delete_bulk_handler, message_delete_handler, message_update_handler};
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
    model::{event::ResumedEvent, gateway::Ready, prelude::*},
    prelude::*,
};
use sqlx::PgPool;
use std::{collections::HashSet, env, sync::Arc};
use tracing::{error, info, instrument};
use utils::database::{initialize_tables, obtain_pool, run_migrations};

const VERSION: &str = env!("CARGO_PKG_VERSION");

struct ShardManagerContainer;

/// Postgres connection pool.
struct ConnectionPool;

/// Asynchronous client to make HTTP requests.
struct RequestClient;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

impl TypeMapKey for ConnectionPool {
    type Value = PgPool;
}

impl TypeMapKey for RequestClient {
    type Value = reqwest::Client;
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    #[instrument(skip(self, ready))]
    async fn ready(&self, _: Context, ready: Ready) {
        info!("Connected as {}", ready.user.name);
        info!("Version {}", VERSION);
    }

    #[instrument(skip(self))]
    async fn resume(&self, _: Context, _: ResumedEvent) {
        info!("Resumed");
    }

    #[instrument(skip(self, ctx))]
    async fn guild_create(&self, ctx: Context, guild: Guild, is_new: bool) {
        // We'll initialize the database tables for a guild if it's new.
        if is_new {
            initialize_tables(&ctx, &guild).await;
        }
    }

    #[instrument(skip(self, ctx))]
    async fn message_update(
        &self,
        ctx: Context,
        old_if_available: Option<Message>,
        new: Option<Message>,
        event: MessageUpdateEvent,
    ) {
        message_update_handler(ctx, old_if_available, new, event).await;
    }

    #[instrument(skip(self, ctx))]
    async fn message_delete(
        &self,
        ctx: Context,
        channel_id: ChannelId,
        deleted_message_id: MessageId,
        _: Option<GuildId>,
    ) {
        message_delete_handler(ctx, channel_id, deleted_message_id).await;
    }

    #[instrument(skip(self, ctx))]
    async fn message_delete_bulk(
        &self,
        ctx: Context,
        channel_id: ChannelId,
        multiple_deleted_messages_ids: Vec<MessageId>,
        _: Option<GuildId>,
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

/// This is for errors that happen before command execution.
#[hook]
#[instrument]
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
        DispatchError::CheckFailed(_, reason) => {
            match reason {
                Reason::User(r) => {
                    let _ = msg.channel_id.say(ctx, r).await;
                }
                Reason::Log(r) => info!("{}", r),
                Reason::UserAndLog { user, log } => {
                    let _ = msg.channel_id.say(ctx, user).await;
                    info!("{}", log);
                }
                _ => error!("Unknown check error."),
            };
        }
        _ => {
            error!("Unhandled dispatch error: {:?}", error);
        }
    }
}

/// This function executes every time a command finishes executing.
///
/// It's used here to handle errors that happen in the middle of the command.
#[hook]
#[instrument]
async fn after(ctx: &Context, msg: &Message, cmd_name: &str, error: CommandResult) {
    if let Err(why) = &error {
        error!("Error while running command {}", &cmd_name);
        error!("{:?}", &error);

        let err = why.to_string();
        if msg.channel_id.say(ctx, &err).await.is_err() {
            error!(
                "Unable to send messages on channel id {}",
                &msg.channel_id.0
            );
        };
    }
}

/// Sets a custom prefix for a guild.
#[hook]
#[instrument]
pub async fn dynamic_prefix(ctx: &Context, guild_id: &Option<GuildId>) -> Option<String> {
    if let Some(id) = guild_id {
        let data = ctx.data.read().await;
        let pool = data.get::<ConnectionPool>().unwrap();

        let res = sqlx::query!(
            "SELECT prefix FROM prefixes WHERE guild_id = $1",
            id.0 as i64
        )
        .fetch_one(pool)
        .await;

        if let Ok(data) = res {
            if let Some(p) = data.prefix {
                return Some(p);
            }
        } else {
            error!("I couldn't query the database for getting guild prefix.");
        }
    }

    Some("-".to_string())
}

#[tokio::main]
#[instrument]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().expect("Failed to load `.env` file.");
    tracing_subscriber::fmt::init();

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
                .dynamic_prefix(|ctx, msg| {
                    Box::pin(async move { dynamic_prefix(ctx, &msg.guild_id).await })
                })
                .with_whitespace(true)
                .on_mention(Some(bot_id))
                .case_insensitivity(true)
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

    let mut client = Client::builder(&token)
        .framework(framework)
        .event_handler(Handler)
        .intents({
            let mut intents = GatewayIntents::all();
            intents.remove(GatewayIntents::DIRECT_MESSAGE_TYPING);
            intents.remove(GatewayIntents::GUILD_MESSAGE_TYPING);

            intents
        })
        .await
        .expect("Err creating client");

    // Store 50 messages in cache.
    client.cache_and_http.cache.set_max_messages(50).await;

    {
        let mut data = client.data.write().await;
        data.insert::<ShardManagerContainer>(Arc::clone(&client.shard_manager));

        // Add the database connection to the data.
        let pool = obtain_pool(&database_url).await?;
        run_migrations(&pool).await?;
        data.insert::<ConnectionPool>(pool);

        // Add reqwest client to the data.
        let client = reqwest::Client::new();
        data.insert::<RequestClient>(client);
    }

    let shard_manager = client.shard_manager.clone();

    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Could not register ctrl+c handler");
        info!("Shutting down!");
        shard_manager.lock().await.shutdown_all().await;
    });

    if let Err(why) = client.start().await {
        error!("Client error: {:?}", why);
    }

    Ok(())
}
