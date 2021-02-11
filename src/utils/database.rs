//! Obtain database pool from values in the `config.toml` file.

use crate::ConnectionPool;
use serenity::{model::prelude::Guild, prelude::Context};
use sqlx::{
    migrate::Migrator,
    postgres::{PgPool, PgPoolOptions},
};
use std::env;
use tracing::{error, instrument};

static MIGRATOR: Migrator = sqlx::migrate!();

const RUN_MIGRATIONS_FLAGS: (&str, &str) = ("--run-migrations", "-m");

pub async fn obtain_pool(pg_url: &str) -> Result<PgPool, Box<dyn std::error::Error>> {
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(&pg_url)
        .await?;

    Ok(pool)
}

pub async fn run_migrations(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    if env::args().any(|a| a == RUN_MIGRATIONS_FLAGS.0 || a == RUN_MIGRATIONS_FLAGS.1) {
        MIGRATOR.run(pool).await?;
    }

    Ok(())
}

#[instrument(skip(ctx))]
pub async fn initialize_tables(ctx: &Context, guild: &Guild) {
    let data_read = ctx.data.read().await;
    let pool = data_read.get::<ConnectionPool>().unwrap();

    // config table initialization
    if let Err(why) = sqlx::query!(
        r#"
        INSERT INTO config (
            guild_id,
            can_change_na,
            tvmset_lock,
            signups_on,
            total_players,
            total_signups,
            cycle
        ) VALUES (
            $1,
            true,
            false,
            true,
            12,
            0,
            '{ "number": 0, "day": null, "night": null, "votes": null }'
        ) ON CONFLICT (guild_id) DO NOTHING;
        "#,
        guild.id.0 as i64
    )
    .execute(pool)
    .await
    {
        error!(
            "Error initializing config table for guild with ID `{}`: {}",
            guild.id.0, why
        );
    }

    // logging table initialization
    if let Err(why) = sqlx::query!(
        "
        INSERT INTO logging (
            guild_id
        ) VALUES (
            $1
        ) ON CONFLICT (guild_id) DO NOTHING;
        ",
        guild.id.0 as i64
    )
    .execute(pool)
    .await
    {
        error!(
            "Error initializing logging table for guild with ID `{}`: {}",
            guild.id.0, why
        );
    }

    // prefixes table initialization
    if let Err(why) = sqlx::query!(
        "
        INSERT INTO prefixes (
            guild_id, prefix
        ) VALUES (
            $1, '-'
        ) ON CONFLICT (guild_id) DO NOTHING;
        ",
        guild.id.0 as i64
    )
    .execute(pool)
    .await
    {
        error!(
            "Error initializing prefixes table for guild with ID `{}`: {}",
            guild.id.0, why
        );
    }
}
