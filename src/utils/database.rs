// Obtain database pool from values in the `config.toml` file.

use sqlx::postgres::PgPool;

pub async fn obtain_pool(pg_url: &str) -> Result<PgPool, Box<dyn std::error::Error>> {
    let pool = PgPool::builder().max_size(20).build(&pg_url).await?;

    Ok(pool)
}
