use super::{row_activity, row_status};
use crate::ai::agent::conversation::ConversationStatus;
use crate::terminal::CLIAgent;
use crate::terminal::cli_agent_sessions::{
    BackgroundTasks, CLIAgentInputState, CLIAgentSession, CLIAgentSessionContext,
    CLIAgentSessionStatus,
};

fn session(status: CLIAgentSessionStatus, context: CLIAgentSessionContext) -> CLIAgentSession {
    CLIAgentSession {
        agent: CLIAgent::Claude,
        status,
        session_context: context,
        input_state: CLIAgentInputState::Closed,
        should_auto_toggle_input: false,
        listener: None,
        plugin_version: None,
        draft_text: None,
        remote_host: None,
        custom_command_prefix: None,
        // A plugin-backed session, the only kind that has a status to show.
        received_rich_notification: true,
    }
}

#[test]
fn activity_shows_the_running_tool_and_its_target_else_the_summary() {
    let with = |tool: Option<&str>, target: Option<&str>| {
        row_activity(&session(
            CLIAgentSessionStatus::InProgress,
            CLIAgentSessionContext {
                tool_name: tool.map(str::to_owned),
                tool_input_preview: target.map(str::to_owned),
                summary: Some(" Fixing the webhook ".to_owned()),
                ..Default::default()
            },
        ))
    };
    assert_eq!(
        with(Some("Bash"), Some("cargo test")).as_deref(),
        Some("Bash: cargo test")
    );
    assert_eq!(with(Some("Read"), None).as_deref(), Some("Read"));
    assert_eq!(with(None, None).as_deref(), Some("Fixing the webhook"));
}

#[test]
fn status_waits_for_the_first_turn_and_background_work_keeps_it_working() {
    let fresh = session(
        CLIAgentSessionStatus::Success,
        CLIAgentSessionContext::default(),
    );
    assert!(row_status(&fresh).is_none());

    let done_with_agents_running = session(
        CLIAgentSessionStatus::Success,
        CLIAgentSessionContext {
            had_activity: true,
            background_tasks: Some(BackgroundTasks {
                count: 2,
                only_subagents: true,
                shells: 0,
            }),
            ..Default::default()
        },
    );
    assert!(matches!(
        row_status(&done_with_agents_running),
        Some(ConversationStatus::InProgress)
    ));
}
