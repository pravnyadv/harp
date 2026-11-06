//! Harp: a right-hand panel listing the subagents the window's Claude Code sessions started, read
//! from the files Claude Code keeps for them (`cli_agent_sessions::subagents`). A row opens the
//! subagent's recent steps. It works without a Warp account.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use chrono::{DateTime, Utc};
use warpui::elements::{
    ClippedScrollStateHandle, ClippedScrollable, Container, CornerRadius, CrossAxisAlignment,
    DragBarSide, Element, Flex, Hoverable, MainAxisAlignment, MainAxisSize, MouseStateHandle,
    Padding, ParentElement, Radius, Resizable, ResizableStateHandle, ScrollbarWidth, Shrinkable,
    Text, resizable_state_handle,
};
use warpui::fonts::{Properties, Weight};
use warpui::platform::Cursor;
use warpui::text_layout::ClipConfig;
use warpui::{AppContext, SingletonEntity, ViewContext};

use super::vertical_tabs::{render_row_status_icon, strip_agent_title_glyph};
use crate::ai::agent::conversation::ConversationStatus;
use crate::appearance::Appearance;
use crate::drive::panel::{MAX_SIDEBAR_WIDTH_RATIO, MIN_SIDEBAR_WIDTH};
use crate::terminal::CLIAgent;
use crate::terminal::cli_agent_sessions::CLIAgentSessionsModel;
use crate::terminal::cli_agent_sessions::subagents::{
    Step, Subagent, find_subagents_dir, read_steps, read_subagents, session_process_started_at,
};
use crate::workspace::Workspace;
use crate::workspace::action::{AgentsPanelAction, WorkspaceAction};

const POLL_INTERVAL: Duration = Duration::from_secs(1);
/// A finished subagent stays listed this long after its last step.
const FINISHED_VISIBLE_FOR: chrono::Duration = chrono::Duration::minutes(30);
const MAX_FINISHED_PER_SESSION: usize = 5;
const DETAIL_STEPS: usize = 60;

/// A Claude session in the window that has subagents to show.
struct SessionAgents {
    tab_index: usize,
    title: String,
    agents: Vec<Subagent>,
}

/// What the panel reads from the window on each poll, before going to disk.
struct SessionRef {
    session_id: String,
    tab_index: usize,
    title: String,
}

pub(crate) struct ClaudeAgentsPanelState {
    resizable: ResizableStateHandle,
    list_scroll: ClippedScrollStateHandle,
    detail_scroll: ClippedScrollStateHandle,
    row_mouse_states: RefCell<HashMap<String, MouseStateHandle>>,
    back_mouse_state: MouseStateHandle,
    go_to_session_mouse_state: MouseStateHandle,
    sessions: Vec<SessionAgents>,
    /// Each session's `subagents` folder, once found.
    subagent_dirs: HashMap<String, PathBuf>,
    /// The subagent whose steps are shown, by id.
    selected: Option<String>,
    steps: Vec<Step>,
    is_polling: bool,
}

impl Default for ClaudeAgentsPanelState {
    fn default() -> Self {
        Self {
            resizable: resizable_state_handle(320.),
            list_scroll: Default::default(),
            detail_scroll: Default::default(),
            row_mouse_states: Default::default(),
            back_mouse_state: Default::default(),
            go_to_session_mouse_state: Default::default(),
            sessions: Vec::new(),
            subagent_dirs: HashMap::new(),
            selected: None,
            steps: Vec::new(),
            is_polling: false,
        }
    }
}

impl ClaudeAgentsPanelState {
    fn row_mouse_state(&self, agent_id: &str) -> MouseStateHandle {
        self.row_mouse_states
            .borrow_mut()
            .entry(agent_id.to_owned())
            .or_default()
            .clone()
    }

    fn selected_agent(&self) -> Option<(&SessionAgents, &Subagent)> {
        let selected = self.selected.as_deref()?;
        self.sessions.iter().find_map(|session| {
            let agent = session.agents.iter().find(|agent| agent.id == selected)?;
            Some((session, agent))
        })
    }
}

fn claude_dir() -> Option<PathBuf> {
    std::env::var_os("CLAUDE_CONFIG_DIR")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|home| home.join(".claude")))
}

/// Running subagents and the few that finished lately, so a quick one does not vanish unseen.
fn visible_agents(agents: Vec<Subagent>, now: DateTime<Utc>) -> Vec<Subagent> {
    let mut finished_shown = 0;
    agents
        .into_iter()
        .filter(|agent| {
            if !agent.finished {
                return true;
            }
            let recent = agent
                .last_active_at
                .is_some_and(|at| now - at < FINISHED_VISIBLE_FOR);
            finished_shown += usize::from(recent);
            recent && finished_shown <= MAX_FINISHED_PER_SESSION
        })
        .collect()
}

impl Workspace {
    pub(super) fn handle_agents_panel_action(
        &mut self,
        action: &AgentsPanelAction,
        ctx: &mut ViewContext<Self>,
    ) {
        match action {
            AgentsPanelAction::Toggle => {
                self.claude_agents_panel_open = !self.claude_agents_panel_open;
                if self.claude_agents_panel_open && !self.claude_agents_panel.is_polling {
                    self.claude_agents_panel.is_polling = true;
                    self.poll_agents_panel(ctx);
                }
            }
            AgentsPanelAction::OpenAgent(agent_id) => {
                self.claude_agents_panel.selected = Some(agent_id.clone());
                self.claude_agents_panel.steps.clear();
            }
            AgentsPanelAction::Back => {
                self.claude_agents_panel.selected = None;
                self.claude_agents_panel.steps.clear();
            }
            AgentsPanelAction::GoToTab(tab_index) => {
                ctx.dispatch_typed_action(&WorkspaceAction::ActivateTab(*tab_index));
            }
        }
        ctx.notify();
    }

    fn agent_session_refs(&self, app: &AppContext) -> Vec<SessionRef> {
        let sessions = CLIAgentSessionsModel::as_ref(app);
        let mut refs = Vec::new();
        // Only the tab in view: the panel follows the session you are working in.
        let tab_index = self.active_tab_index;
        if let Some(tab) = self.tabs.get(tab_index) {
            for terminal_view in tab.pane_group.as_ref(app).terminal_views(app) {
                let terminal_view = terminal_view.as_ref(app);
                let Some(session) = sessions.session(terminal_view.id()) else {
                    continue;
                };
                if session.agent != CLIAgent::Claude {
                    continue;
                }
                let Some(session_id) = session.session_context.session_id.clone() else {
                    continue;
                };
                let configuration = terminal_view.pane_configuration().as_ref(app);
                let title = configuration
                    .vertical_tabs_label()
                    .unwrap_or_else(|| configuration.title());
                refs.push(SessionRef {
                    session_id,
                    tab_index,
                    title: strip_agent_title_glyph(session.agent, title.trim()).to_owned(),
                });
            }
        }
        refs
    }

    /// Reads the subagent files off the main thread, then polls again while the panel is open.
    fn poll_agents_panel(&mut self, ctx: &mut ViewContext<Self>) {
        let refs = self.agent_session_refs(ctx);
        let mut dirs = self.claude_agents_panel.subagent_dirs.clone();
        let selected = self.claude_agents_panel.selected.clone();
        ctx.spawn(
            async move {
                let claude_dir = claude_dir();
                let projects = claude_dir.as_ref().map(|dir| dir.join("projects"));
                let registry = claude_dir.as_ref().map(|dir| dir.join("sessions"));
                let now = Utc::now();
                let mut sessions = Vec::new();
                for session in refs {
                    let dir = match dirs.get(&session.session_id) {
                        Some(dir) => Some(dir.clone()),
                        None => projects
                            .as_deref()
                            .and_then(|root| find_subagents_dir(root, &session.session_id)),
                    };
                    let Some(dir) = dir else { continue };
                    dirs.insert(session.session_id.clone(), dir.clone());
                    let process_started_at = registry.as_deref().and_then(|registry| {
                        session_process_started_at(registry, &session.session_id)
                    });
                    let agents = visible_agents(read_subagents(&dir, process_started_at, now), now);
                    if !agents.is_empty() {
                        sessions.push(SessionAgents {
                            tab_index: session.tab_index,
                            title: session.title,
                            agents,
                        });
                    }
                }
                let steps = selected
                    .and_then(|id| {
                        sessions
                            .iter()
                            .flat_map(|session| &session.agents)
                            .find(|agent| agent.id == id)
                    })
                    .map(|agent| read_steps(&agent.transcript, DETAIL_STEPS))
                    .unwrap_or_default();
                (sessions, dirs, steps)
            },
            |me, (sessions, dirs, steps), ctx| {
                let state = &mut me.claude_agents_panel;
                state.sessions = sessions;
                state.subagent_dirs = dirs;
                state.steps = steps;
                ctx.notify();
                if !me.claude_agents_panel_open {
                    me.claude_agents_panel.is_polling = false;
                    return;
                }
                ctx.spawn(
                    warpui::r#async::Timer::after(POLL_INTERVAL),
                    |me, _, ctx| {
                        me.poll_agents_panel(ctx);
                    },
                );
            },
        );
    }

    pub(super) fn render_claude_agents_panel(&self, app: &AppContext) -> Box<dyn Element> {
        let state = &self.claude_agents_panel;
        let content = match state.selected_agent() {
            Some((session, agent)) => render_detail(state, session, agent, app),
            None => render_list(state, app),
        };
        Resizable::new(state.resizable.clone(), content)
            .with_dragbar_side(DragBarSide::Left)
            .on_resize(|ctx, _| ctx.notify())
            .with_bounds_callback(Box::new(|window_size| {
                let max = window_size.x() * MAX_SIDEBAR_WIDTH_RATIO;
                (MIN_SIDEBAR_WIDTH, max.max(MIN_SIDEBAR_WIDTH))
            }))
            .finish()
    }
}

fn agent_status(agent: &Subagent) -> ConversationStatus {
    if agent.finished {
        ConversationStatus::Success
    } else {
        ConversationStatus::InProgress
    }
}

fn format_elapsed(agent: &Subagent, now: DateTime<Utc>) -> Option<String> {
    let end = if agent.finished {
        agent.last_active_at?
    } else {
        now
    };
    let seconds = (end - agent.started_at?).num_seconds().max(0);
    Some(match seconds {
        0..60 => format!("{seconds}s"),
        60..3600 => format!("{}m {}s", seconds / 60, seconds % 60),
        _ => format!("{}h {}m", seconds / 3600, seconds / 60 % 60),
    })
}

/// "Explore · haiku · background · 1m 20s"
fn agent_meta_line(agent: &Subagent, now: DateTime<Utc>) -> String {
    [
        Some(agent.agent_type.clone()),
        agent.model.clone(),
        agent.background.then(|| "background".to_owned()),
        format_elapsed(agent, now),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" · ")
}

fn agent_title(agent: &Subagent) -> String {
    if agent.description.is_empty() {
        agent.agent_type.clone()
    } else {
        agent.description.clone()
    }
}

fn header(title: String, trailing: Option<String>, app: &AppContext) -> Box<dyn Element> {
    let appearance = Appearance::as_ref(app);
    let theme = appearance.theme();
    let mut row = Flex::row()
        .with_main_axis_size(MainAxisSize::Max)
        .with_main_axis_alignment(MainAxisAlignment::SpaceBetween)
        .with_child(
            Shrinkable::new(
                1.,
                Text::new_inline(
                    title,
                    appearance.ui_font_family(),
                    appearance.ui_font_size(),
                )
                .with_style(Properties::default().weight(Weight::Semibold))
                .with_clip(ClipConfig::ellipsis())
                .with_color(theme.main_text_color(theme.background()).into())
                .finish(),
            )
            .finish(),
        );
    if let Some(trailing) = trailing {
        row.add_child(
            Text::new_inline(
                trailing,
                appearance.ui_font_family(),
                appearance.ui_font_size() - 1.,
            )
            .with_color(theme.sub_text_color(theme.background()).into())
            .finish(),
        );
    }
    Container::new(row.finish())
        .with_padding(Padding::uniform(12.))
        .finish()
}

fn scrollable(
    scroll_state: &ClippedScrollStateHandle,
    content: Box<dyn Element>,
    app: &AppContext,
) -> Box<dyn Element> {
    let theme = Appearance::as_ref(app).theme();
    ClippedScrollable::vertical(
        scroll_state.clone(),
        content,
        ScrollbarWidth::Auto,
        theme.nonactive_ui_detail().into(),
        theme.active_ui_detail().into(),
        theme.background().into(),
    )
    .finish()
}

fn clickable(
    mouse_state: MouseStateHandle,
    content: Box<dyn Element>,
    action: AgentsPanelAction,
    app: &AppContext,
) -> Box<dyn Element> {
    let surface = Appearance::as_ref(app).theme().surface_2();
    Hoverable::new(mouse_state, move |mouse| {
        let mut container = Container::new(content)
            .with_padding(Padding::uniform(8.))
            .with_horizontal_margin(6.)
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(8.)));
        if mouse.is_hovered() {
            container = container.with_background(surface);
        }
        container.finish()
    })
    .with_cursor(Cursor::PointingHand)
    .on_click(move |ctx, _, _| {
        ctx.dispatch_typed_action(WorkspaceAction::AgentsPanel(action.clone()));
    })
    .finish()
}

fn render_list(state: &ClaudeAgentsPanelState, app: &AppContext) -> Box<dyn Element> {
    let appearance = Appearance::as_ref(app);
    let theme = appearance.theme();
    let font = appearance.ui_font_family();
    let size = appearance.ui_font_size();
    let main = theme.main_text_color(theme.background());
    let sub = theme.sub_text_color(theme.background());
    let now = Utc::now();

    let running = state
        .sessions
        .iter()
        .flat_map(|session| &session.agents)
        .filter(|agent| !agent.finished)
        .count();
    let mut column = Flex::column()
        .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_child(header(
            "Agents".to_owned(),
            (running > 0).then(|| format!("{running} running")),
            app,
        ));

    let mut list = Flex::column().with_cross_axis_alignment(CrossAxisAlignment::Stretch);
    if state.sessions.is_empty() {
        list.add_child(
            Container::new(
                Text::new(
                    "No subagents in this window's Claude sessions. When Claude starts one, \
                     it shows up here while it runs."
                        .to_owned(),
                    font,
                    size - 1.,
                )
                .with_color(sub.into())
                .finish(),
            )
            .with_padding(Padding::uniform(12.))
            .finish(),
        );
    }
    for session in &state.sessions {
        list.add_child(
            Container::new(
                Text::new_inline(session.title.clone(), font, size - 2.)
                    .with_style(Properties::default().weight(Weight::Semibold))
                    .with_clip(ClipConfig::ellipsis())
                    .with_color(sub.into())
                    .finish(),
            )
            .with_padding(
                Padding::uniform(0.)
                    .with_left(14.)
                    .with_right(14.)
                    .with_top(10.),
            )
            .with_margin_bottom(2.)
            .finish(),
        );
        for agent in &session.agents {
            let title_color = if agent.finished { sub } else { main };
            let mut body = Flex::column()
                .with_child(
                    Text::new_inline(agent_title(agent), font, size)
                        .with_clip(ClipConfig::ellipsis())
                        .with_color(title_color.into())
                        .finish(),
                )
                .with_child(
                    Text::new_inline(agent_meta_line(agent, now), font, size - 2.)
                        .with_clip(ClipConfig::ellipsis())
                        .with_color(sub.into())
                        .finish(),
                );
            if let Some(step) = agent.current_step.clone().filter(|_| !agent.finished) {
                body.add_child(
                    Text::new_inline(step, font, size - 2.)
                        .with_clip(ClipConfig::ellipsis())
                        .with_color(sub.into())
                        .finish(),
                );
            }
            let content = Flex::row()
                .with_cross_axis_alignment(CrossAxisAlignment::Start)
                .with_child(
                    Container::new(render_row_status_icon(&agent_status(agent), appearance))
                        .with_margin_right(6.)
                        .finish(),
                )
                .with_child(Shrinkable::new(1., body.finish()).finish())
                .finish();
            list.add_child(clickable(
                state.row_mouse_state(&agent.id),
                content,
                AgentsPanelAction::OpenAgent(agent.id.clone()),
                app,
            ));
        }
    }
    column.add_child(
        Shrinkable::new(1., scrollable(&state.list_scroll, list.finish(), app)).finish(),
    );
    column.finish()
}

fn render_detail(
    state: &ClaudeAgentsPanelState,
    session: &SessionAgents,
    agent: &Subagent,
    app: &AppContext,
) -> Box<dyn Element> {
    let appearance = Appearance::as_ref(app);
    let theme = appearance.theme();
    let font = appearance.ui_font_family();
    let size = appearance.ui_font_size();
    let main = theme.main_text_color(theme.background());
    let sub = theme.sub_text_color(theme.background());
    let accent = theme.accent();
    let small = |text: String, color: warp_core::ui::theme::Fill| {
        Text::new_inline(text, font, size - 1.)
            .with_clip(ClipConfig::ellipsis())
            .with_color(color.into())
            .finish()
    };

    let nav = Flex::row()
        .with_main_axis_size(MainAxisSize::Max)
        .with_main_axis_alignment(MainAxisAlignment::SpaceBetween)
        .with_child(clickable(
            state.back_mouse_state.clone(),
            small("‹ Agents".to_owned(), accent),
            AgentsPanelAction::Back,
            app,
        ))
        .with_child(
            Shrinkable::new(
                1.,
                clickable(
                    state.go_to_session_mouse_state.clone(),
                    small(format!("Go to {}", session.title), accent),
                    AgentsPanelAction::GoToTab(session.tab_index),
                    app,
                ),
            )
            .finish(),
        )
        .finish();

    let summary = Flex::row()
        .with_cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(
            Container::new(render_row_status_icon(&agent_status(agent), appearance))
                .with_margin_right(6.)
                .finish(),
        )
        .with_child(
            Shrinkable::new(
                1.,
                Flex::column()
                    .with_child(
                        Text::new(agent_title(agent), font, size)
                            .with_style(Properties::default().weight(Weight::Semibold))
                            .with_color(main.into())
                            .finish(),
                    )
                    .with_child(small(agent_meta_line(agent, Utc::now()), sub))
                    .finish(),
            )
            .finish(),
        )
        .finish();

    let mut steps = Flex::column().with_cross_axis_alignment(CrossAxisAlignment::Stretch);
    if state.steps.is_empty() {
        steps.add_child(small("No steps yet.".to_owned(), sub));
    }
    for step in &state.steps {
        let at = step
            .at
            .map(|at| {
                at.with_timezone(&chrono::Local)
                    .format("%H:%M:%S")
                    .to_string()
            })
            .unwrap_or_default();
        steps.add_child(
            Container::new(
                Flex::row()
                    .with_cross_axis_alignment(CrossAxisAlignment::Start)
                    .with_child(
                        Container::new(
                            Text::new_inline(at, appearance.monospace_font_family(), size - 2.)
                                .with_color(sub.into())
                                .finish(),
                        )
                        .with_margin_right(8.)
                        .finish(),
                    )
                    .with_child(
                        Shrinkable::new(
                            1.,
                            Text::new(step.kind.label(), font, size - 1.)
                                .with_color(main.into())
                                .finish(),
                        )
                        .finish(),
                    )
                    .finish(),
            )
            .with_padding(Padding::uniform(0.).with_top(4.).with_bottom(4.))
            .finish(),
        );
    }

    Flex::column()
        .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_child(Container::new(nav).with_margin_top(6.).finish())
        .with_child(
            Container::new(summary)
                .with_padding(Padding::uniform(12.))
                .finish(),
        )
        .with_child(
            Shrinkable::new(
                1.,
                scrollable(
                    &state.detail_scroll,
                    Container::new(steps.finish())
                        .with_padding(Padding::uniform(0.).with_left(14.).with_right(14.))
                        .finish(),
                    app,
                ),
            )
            .finish(),
        )
        .finish()
}

#[cfg(test)]
#[path = "claude_agents_panel_tests.rs"]
mod tests;
