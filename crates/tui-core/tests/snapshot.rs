#![allow(unused_must_use)]
use deepseek_tui_core::{Pane, UiEvent, UiState};

#[test]
fn reducer_produces_stable_snapshot_for_core_workflow() {
    let mut state = UiState::default();
    state.reduce(UiEvent::PromptSubmitted("hello".to_string()));
    state.reduce(UiEvent::ToolStarted("web.search".to_string()));
    state.reduce(UiEvent::ResponseDelta("partial".to_string()));
    state.reduce(UiEvent::ToolFinished("web.search".to_string()));
    state.reduce(UiEvent::ApprovalRequested("approval-1".to_string()));
    state.reduce(UiEvent::ApprovalResolved("approval-1".to_string()));
    state.reduce(UiEvent::JobQueued("job-1".to_string()));
    state.reduce(UiEvent::JobProgress {
        job_id: "job-1".to_string(),
        progress: 60,
    });
    state.reduce(UiEvent::JobCompleted("job-1".to_string()));
    state.reduce(UiEvent::KeyPressed('5'));

    assert_eq!(state.active_pane, Pane::Jobs);
    assert_eq!(
        state.snapshot(),
        "pane=jobs;paused=false;pending_tasks=0;active_jobs=0;pending_approvals=0;active_tool=;status=job completed;events=10"
    );
}

#[test]
fn reducer_handles_progress_clamping() {
    let mut state = UiState::default();
    // Out-of-range progress should be clamped
    state.reduce(UiEvent::JobProgress {
        job_id: "j".into(),
        progress: 255,
    });
    assert_eq!(state.status_line, "job progress: 100%");

    state.reduce(UiEvent::JobProgress {
        job_id: "j".into(),
        progress: 0,
    });
    assert_eq!(state.status_line, "job progress: 0%");
}

#[test]
fn state_json_round_trip() {
    let mut state = UiState::default();
    state.reduce(UiEvent::ToolStarted("read_file".into()));
    state.reduce(UiEvent::ResponseDelta("some text".into()));

    let json = state.to_json().expect("serialize");
    let restored = UiState::from_json(&json).expect("deserialize");

    assert_eq!(restored.active_tool, Some("read_file".into()));
    assert_eq!(
        restored.last_response_delta,
        Some("some text".into())
    );
    assert_eq!(restored.event_count, 2);
}

#[test]
fn unknown_key_does_not_change_pane() {
    let mut state = UiState::default();
    assert_eq!(state.active_pane, Pane::Chat);
    state.reduce(UiEvent::KeyPressed('9'));
    assert_eq!(state.active_pane, Pane::Chat);
}