use takokit_core::RuntimeConfig;
use takokit_store::LocalStore;

pub async fn open_gui(store: &LocalStore, config: &RuntimeConfig) -> anyhow::Result<()> {
    ensure_server(store, config).await?;

    let url = config.gui_url();
    match open::that(&url) {
        Ok(()) => println!("Opened Takokit local web GUI at {url}"),
        Err(error) => {
            println!("Takokit local web GUI: {url}");
            eprintln!("Could not open the browser automatically: {error}");
        }
    }

    Ok(())
}

pub async fn ensure_server(store: &LocalStore, config: &RuntimeConfig) -> anyhow::Result<()> {
    let _ = crate::daemon::ensure_running(store, config)?;
    Ok(())
}

pub fn gui_dist_path() -> std::path::PathBuf {
    std::env::var("TAKOKIT_GUI_DIST")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../gui/dist")
        })
}
