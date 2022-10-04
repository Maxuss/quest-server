use sqlx::PgPool;

#[allow(unused_variables)]
#[tracing::instrument(skip_all)]
pub async fn start_telegram(token: String, pool: PgPool) -> anyhow::Result<()> {
    tracing::info!("Starting telegram bot");

    Ok(())
}
