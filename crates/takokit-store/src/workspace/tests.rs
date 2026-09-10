use super::*;
use takokit_core::{SessionEventState, SessionTask};

#[test]
fn listing_empty_workspace_does_not_create_tako() {
    let root = std::env::temp_dir().join(format!("takokit-empty-workspace-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let store = WorkspaceStore::new(&root);
    assert!(store.list_sessions(None).unwrap().is_empty());
    assert!(!root.join(".tako").exists());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn workspace_sessions_persist_events_outputs_and_search() {
    let root = std::env::temp_dir().join(format!("takokit-workspace-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&root).expect("workspace");
    let store = WorkspaceStore::new(&root);
    let session = store.create_session(None).expect("session");
    let output = store
        .write_text_output(session.summary.id, "transcript.txt", "hello world")
        .expect("output");
    store
        .append_event(
            session.summary.id,
            NewSessionEvent {
                task: SessionTask::SpeechToText,
                state: SessionEventState::Completed,
                model: Some("whisper-tiny".into()),
                input: None,
                source_path: Some(root.join("audio.wav")),
                output_path: Some(output),
                text: Some("hello world".into()),
                message: None,
            },
        )
        .expect("event");
    let record = store.read_session(session.summary.id).expect("record");
    assert_eq!(record.events.len(), 1);
    assert_eq!(record.summary.output_count, 1);
    assert_eq!(store.list_sessions(Some("hello world")).unwrap().len(), 1);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn summary_replacement_supports_multiple_updates() {
    let root = std::env::temp_dir().join(format!("takokit-replace-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let store = WorkspaceStore::new(&root);
    let session = store.create_session(Some("one")).unwrap();
    store.set_active_session(session.summary.id).unwrap();
    store.set_active_session(session.summary.id).unwrap();
    assert_eq!(store.active_session().unwrap(), Some(session.summary.id));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn stale_summary_is_reconciled_from_durable_events() {
    let root = tempfile::tempdir().unwrap();
    let store = WorkspaceStore::new(root.path());
    let session = store.create_session(Some("keep title")).unwrap();
    let event = SessionEvent {
        id: Uuid::new_v4(),
        session_id: session.summary.id,
        timestamp: session.summary.created_at + 10,
        task: SessionTask::SpeechToText,
        state: SessionEventState::Completed,
        model: Some("whisper-tiny".into()),
        input: None,
        source_path: None,
        output_path: Some(PathBuf::from("out.txt")),
        text: Some("hello".into()),
        message: None,
    };
    let mut encoded = serde_json::to_vec(&event).unwrap();
    encoded.push(b'\n');
    std::fs::write(
        store.session_dir(session.summary.id).join("events.jsonl"),
        encoded,
    )
    .unwrap();

    let recovered = store.read_session(session.summary.id).unwrap();
    assert_eq!(recovered.summary.event_count, 1);
    assert_eq!(recovered.summary.output_count, 1);
    assert_eq!(recovered.summary.last_task, Some(SessionTask::SpeechToText));
    assert_eq!(
        recovered.summary.last_model.as_deref(),
        Some("whisper-tiny")
    );
    assert_eq!(recovered.summary.title, "keep title");
}

#[test]
fn default_title_is_recovered_from_first_durable_event() {
    let root = tempfile::tempdir().unwrap();
    let store = WorkspaceStore::new(root.path());
    let session = store.create_session(None).unwrap();
    let event = SessionEvent {
        id: Uuid::new_v4(),
        session_id: session.summary.id,
        timestamp: session.summary.created_at + 1,
        task: SessionTask::SpeechToText,
        state: SessionEventState::Completed,
        model: Some("whisper-tiny".into()),
        input: None,
        source_path: None,
        output_path: None,
        text: None,
        message: None,
    };
    let mut encoded = serde_json::to_vec(&event).unwrap();
    encoded.push(b'\n');
    std::fs::write(
        store.session_dir(session.summary.id).join("events.jsonl"),
        encoded,
    )
    .unwrap();

    let recovered = store.read_session(session.summary.id).unwrap();
    assert_eq!(recovered.summary.title, "Speech to text · whisper-tiny");
}

#[test]
fn torn_final_event_is_truncated_without_losing_valid_events() {
    let root = tempfile::tempdir().unwrap();
    let store = WorkspaceStore::new(root.path());
    let session = store.create_session(None).unwrap();
    store
        .append_event(
            session.summary.id,
            NewSessionEvent {
                task: SessionTask::Diagnostics,
                state: SessionEventState::Completed,
                model: None,
                input: None,
                source_path: None,
                output_path: None,
                text: None,
                message: Some("ok".into()),
            },
        )
        .unwrap();
    let events_path = store.session_dir(session.summary.id).join("events.jsonl");
    let mut file = OpenOptions::new().append(true).open(&events_path).unwrap();
    file.write_all(br#"{"id":"torn""#).unwrap();
    file.sync_all().unwrap();

    let recovered = store.read_session(session.summary.id).unwrap();
    assert_eq!(recovered.events.len(), 1);
    assert!(std::fs::read(&events_path).unwrap().ends_with(b"\n"));
}

#[test]
fn missing_primary_summary_recovers_from_backup() {
    let root = tempfile::tempdir().unwrap();
    let store = WorkspaceStore::new(root.path());
    let session = store.create_session(Some("recover me")).unwrap();
    let path = store.summary_path(session.summary.id);
    let backup = recovery::backup_path(&path);
    std::fs::copy(&path, &backup).unwrap();
    std::fs::remove_file(&path).unwrap();

    let recovered = store.read_session(session.summary.id).unwrap();
    assert_eq!(recovered.summary.title, "recover me");
    assert!(path.is_file());
    assert!(!backup.exists());
}

#[test]
fn corrupt_primary_summary_recovers_from_valid_backup() {
    let root = tempfile::tempdir().unwrap();
    let store = WorkspaceStore::new(root.path());
    let session = store.create_session(Some("recover me")).unwrap();
    let path = store.summary_path(session.summary.id);
    let backup = recovery::backup_path(&path);
    std::fs::copy(&path, &backup).unwrap();
    std::fs::write(&path, b"{torn").unwrap();

    let recovered = store.read_session(session.summary.id).unwrap();
    assert_eq!(recovered.summary.title, "recover me");
    assert!(serde_json::from_slice::<SessionSummary>(&std::fs::read(&path).unwrap()).is_ok());
    assert!(!backup.exists());
}

#[test]
fn mismatched_primary_summary_recovers_from_valid_backup() {
    let root = tempfile::tempdir().unwrap();
    let store = WorkspaceStore::new(root.path());
    let session = store.create_session(Some("recover identity")).unwrap();
    let path = store.summary_path(session.summary.id);
    let backup = recovery::backup_path(&path);
    std::fs::copy(&path, &backup).unwrap();
    let mut wrong = session.summary.clone();
    wrong.id = Uuid::new_v4();
    std::fs::write(&path, serde_json::to_vec_pretty(&wrong).unwrap()).unwrap();

    let recovered = store.read_session(session.summary.id).unwrap();
    assert_eq!(recovered.summary.id, session.summary.id);
    assert_eq!(recovered.summary.title, "recover identity");
}

#[test]
fn corrupt_active_session_recovers_from_valid_backup() {
    let root = tempfile::tempdir().unwrap();
    let store = WorkspaceStore::new(root.path());
    let session = store.create_session(Some("active")).unwrap();
    let path = store.root().join("active-session");
    let backup = recovery::backup_path(&path);
    std::fs::copy(&path, &backup).unwrap();
    std::fs::write(&path, b"torn").unwrap();

    assert_eq!(store.active_session().unwrap(), Some(session.summary.id));
    assert_eq!(
        std::fs::read_to_string(&path).unwrap().trim(),
        session.summary.id.to_string()
    );
    assert!(!backup.exists());
}

#[test]
fn removed_active_session_does_not_resurrect_from_backup() {
    let root = tempfile::tempdir().unwrap();
    let store = WorkspaceStore::new(root.path());
    let session = store.create_session(Some("remove")).unwrap();
    let path = store.root().join("active-session");
    let backup = recovery::backup_path(&path);
    std::fs::copy(&path, &backup).unwrap();

    assert!(store.remove_session(session.summary.id).unwrap());
    assert_eq!(store.active_session().unwrap(), None);
    assert!(!path.exists());
    assert!(!backup.exists());
}

#[test]
fn foreign_session_event_is_rejected() {
    let root = tempfile::tempdir().unwrap();
    let store = WorkspaceStore::new(root.path());
    let session = store.create_session(Some("one")).unwrap();
    let other = Uuid::new_v4();
    let event = SessionEvent {
        id: Uuid::new_v4(),
        session_id: other,
        timestamp: session.summary.created_at + 1,
        task: SessionTask::Diagnostics,
        state: SessionEventState::Completed,
        model: None,
        input: None,
        source_path: None,
        output_path: None,
        text: None,
        message: None,
    };
    let mut encoded = serde_json::to_vec(&event).unwrap();
    encoded.push(b'\n');
    std::fs::write(
        store.session_dir(session.summary.id).join("events.jsonl"),
        encoded,
    )
    .unwrap();

    assert!(store.read_session(session.summary.id).is_err());
}
