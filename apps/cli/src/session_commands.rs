use std::path::PathBuf;

use crate::{args::SessionsCommand, workspace::resolve_store};

pub(crate) fn run_sessions_command(
    workspace: Option<PathBuf>,
    command: SessionsCommand,
) -> anyhow::Result<()> {
    let store = resolve_store(workspace)?;
    match command {
        SessionsCommand::List { query } => {
            let sessions = store.list_sessions(query.as_deref())?;
            println!("{}", serde_json::to_string_pretty(&sessions)?);
        }
        SessionsCommand::New { title } => {
            let session = store.create_session(title.as_deref())?;
            println!("{}", serde_json::to_string_pretty(&session)?);
        }
        SessionsCommand::Show { id } => {
            let session = store.read_session(id)?;
            println!("{}", serde_json::to_string_pretty(&session)?);
        }
        SessionsCommand::Open { id } => {
            let session = store.read_session(id)?;
            store.set_active_session(id)?;
            println!("{}", serde_json::to_string_pretty(&session)?);
        }
        SessionsCommand::Rm { id } => {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "id": id,
                    "removed": store.remove_session(id)?
                }))?
            );
        }
    }
    Ok(())
}
