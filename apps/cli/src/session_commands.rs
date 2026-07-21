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
            crate::print_serializable(&sessions)?;
        }
        SessionsCommand::New { title } => {
            let session = store.create_session(title.as_deref())?;
            crate::print_serializable(&session)?;
        }
        SessionsCommand::Show { id } => {
            let session = store.read_session(id)?;
            crate::print_serializable(&session)?;
        }
        SessionsCommand::Open { id } => {
            let session = store.read_session(id)?;
            store.set_active_session(id)?;
            crate::print_serializable(&session)?;
        }
        SessionsCommand::Rm { id } => {
            crate::print_value(&serde_json::json!({
                "id": id,
                "removed": store.remove_session(id)?
            }))?;
        }
    }
    Ok(())
}
