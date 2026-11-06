use std::path::PathBuf;

use chrono::{DateTime, Duration, Utc};

use super::{agent_meta_line, visible_agents};
use crate::terminal::cli_agent_sessions::subagents::Subagent;

fn agent(id: &str, finished: bool, last_active_minutes_ago: i64, now: DateTime<Utc>) -> Subagent {
    Subagent {
        id: id.to_owned(),
        agent_type: "Explore".to_owned(),
        description: format!("Agent {id}"),
        model: Some("haiku".to_owned()),
        background: true,
        finished,
        started_at: Some(now - Duration::minutes(last_active_minutes_ago) - Duration::seconds(80)),
        last_active_at: Some(now - Duration::minutes(last_active_minutes_ago)),
        current_step: None,
        transcript: PathBuf::new(),
    }
}

#[test]
fn running_agents_stay_and_finished_ones_fade_after_a_while() {
    let now = Utc::now();
    let mut agents = vec![agent("run", false, 120, now)];
    agents.extend((0..7).map(|i| agent(&format!("done{i}"), true, i, now)));
    agents.push(agent("stale", true, 45, now));

    let shown: Vec<String> = visible_agents(agents, now)
        .into_iter()
        .map(|agent| agent.id)
        .collect();
    assert_eq!(shown, ["run", "done0", "done1", "done2", "done3", "done4"]);
}

#[test]
fn meta_line_names_type_model_shape_and_elapsed() {
    let now = Utc::now();
    assert_eq!(
        agent_meta_line(&agent("a", true, 3, now), now),
        "Explore · haiku · background · 1m 20s"
    );
    let mut running = agent("b", false, 0, now);
    running.started_at = Some(now - Duration::seconds(42));
    running.background = false;
    running.model = None;
    assert_eq!(agent_meta_line(&running, now), "Explore · 42s");
}
