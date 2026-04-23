use restsend_backend::app::{build_router, init_tracing, AppConfig};

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("restsend-backend failed: {err}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenvy::dotenv();

    let config = AppConfig::from_env();
    let _tracing_guard = init_tracing(&config);

    let (app, state) = build_router(config.clone()).await?;

    if config.migrate_only {
        tracing::info!("migration-only mode completed");
        return Ok(());
    }

    let listener = tokio::net::TcpListener::bind(&config.addr)
        .await
        .map_err(|err| format!("failed to bind {}: {}", config.addr, err))?;
    tracing::info!(addr = %config.addr, "restsend backend listening");

    axum::serve(listener, app.with_state(state)).await?;
    Ok(())
}
