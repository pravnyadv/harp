//! Harp: a right-hand panel listing every CLI agent session (Claude Code, Codex, ...) in the
//! window, with what each is doing now. Rows come from `CLIAgentSessionsModel`, so it works
//! without a Warp account.

use std::cell::RefCell;
use std::collections::HashMap;

use warpui::elements::{
    ClippedScrollStateHandle, ClippedScrollable, ConstrainedBox, Container, CornerRadius,
    CrossAxisAlignment, Element, Flex, Hoverable, MouseStateHandle, Padding, ParentElement, Radius,
    ScrollbarWidth, Shrinkable, Text,
};
use warpui::fonts::{Properties, Weight};
use warpui::platform::Cursor;
use warpui::text_layout::ClipConfig;
use warpui::{AppContext, EntityId, SingletonEntity};

use super::vertical_tabs::{background_tasks_label, render_row_status_icon};
use crate::ai::agent::conversation::ConversationStatus;
use crate::appearance::Appearance;
use crate::tab::TabData;
use crate::terminal::CLIAgent;
use crate::terminal::cli_agent_sessions::{CLIAgentSession, CLIAgentSessionsModel};
use crate::workspace::action::WorkspaceAction;

const PANEL_WIDTH: f32 = 320.;

/// One agent session as the panel shows it.
struct AgentRow {
    terminal_view_id: EntityId,
    tab_index: usize,
    is_active_tab: bool,
    title: String,
    /// `None` until the agent reports its first turn.
    status: Option<ConversationStatus>,
    /// The tool it is running now, with its target when the plugin sent one.
    activity: Option<String>,
    prompt: Option<String>,
    background: Option<String>,
}

/// Mouse and scroll state that must outlive a render. Row handles are created the first time a
/// session is shown and kept for it.
#[derive(Default)]
pub(crate) struct ClaudeAgentsPanelState {
    scroll_state: ClippedScrollStateHandle,
    row_mouse_states: RefCell<HashMap<EntityId, MouseStateHandle>>,
}

impl ClaudeAgentsPanelState {
    fn row_mouse_state(&self, terminal_view_id: EntityId) -> MouseStateHandle {
        self.row_mouse_states
            .borrow_mut()
            .entry(terminal_view_id)
            .or_default()
            .clone()
    }
}

fn agent_rows(tabs: &[TabData], active_tab_index: usize, app: &AppContext) -> Vec<AgentRow> {
    let sessions = CLIAgentSessionsModel::as_ref(app);
    let mut rows = Vec::new();
    for (tab_index, tab) in tabs.iter().enumerate() {
        for terminal_view in tab.pane_group.as_ref(app).terminal_views(app) {
            let terminal_view = terminal_view.as_ref(app);
            let Some(session) = sessions
                .session(terminal_view.id())
                .filter(|session| !matches!(session.agent, CLIAgent::Unknown))
            else {
                continue;
            };
            let configuration = terminal_view.pane_configuration().as_ref(app);
            let title = configuration
                .vertical_tabs_label()
                .unwrap_or_else(|| configuration.title())
                .trim()
                .to_owned();
            rows.push(AgentRow {
                terminal_view_id: terminal_view.id(),
                tab_index,
                is_active_tab: tab_index == active_tab_index,
                title,
                status: row_status(session),
                activity: row_activity(session),
                prompt: session.session_context.latest_user_prompt(),
                background: session
                    .session_context
                    .running_background_tasks()
                    .map(background_tasks_label),
            });
        }
    }
    rows
}

fn row_status(session: &CLIAgentSession) -> Option<ConversationStatus> {
    let context = &session.session_context;
    (session.shows_status() && context.had_activity).then(|| {
        session
            .status
            .displayed(context.running_background_tasks().is_some())
            .to_conversation_status()
    })
}

fn row_activity(session: &CLIAgentSession) -> Option<String> {
    let context = &session.session_context;
    match (&context.tool_name, &context.tool_input_preview) {
        (Some(tool), Some(target)) => Some(format!("{tool}: {target}")),
        (Some(tool), None) => Some(tool.clone()),
        (None, _) => context.title_like_text(),
    }
}

pub(super) fn render_claude_agents_panel(
    state: &ClaudeAgentsPanelState,
    tabs: &[TabData],
    active_tab_index: usize,
    app: &AppContext,
) -> Box<dyn Element> {
    let appearance = Appearance::as_ref(app);
    let theme = appearance.theme();
    let font = appearance.ui_font_family();
    let size = appearance.ui_font_size();
    let main = theme.main_text_color(theme.background());
    let sub = theme.sub_text_color(theme.background());

    let header = Container::new(
        Text::new_inline("Agents".to_owned(), font, size)
            .with_style(Properties::default().weight(Weight::Semibold))
            .with_color(main.into())
            .finish(),
    )
    .with_padding(Padding::uniform(12.))
    .finish();

    let rows = agent_rows(tabs, active_tab_index, app);
    let mut list = Flex::column().with_cross_axis_alignment(CrossAxisAlignment::Stretch);
    if rows.is_empty() {
        list.add_child(
            Container::new(
                Text::new(
                    "No agent sessions in this window. Start Claude Code in a tab and it shows \
                     up here."
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
    for row in rows {
        let line = |text: String, color| {
            Text::new_inline(text, font, size - 1.)
                .with_clip(ClipConfig::ellipsis())
                .with_color(color)
                .finish()
        };
        let mut body = Flex::column().with_child(
            Text::new_inline(row.title, font, size)
                .with_clip(ClipConfig::ellipsis())
                .with_color(main.into())
                .finish(),
        );
        for detail in [
            row.activity,
            row.prompt.map(|prompt| format!("You: {prompt}")),
        ]
        .into_iter()
        .flatten()
        {
            body.add_child(line(detail, sub.into()));
        }
        if let Some(background) = row.background {
            body.add_child(line(background, theme.accent().into()));
        }
        let status_icon = match &row.status {
            Some(status) => render_row_status_icon(status, appearance),
            None => ConstrainedBox::new(Flex::row().finish())
                .with_width(16.)
                .finish(),
        };
        let content = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Start)
            .with_child(Container::new(status_icon).with_margin_right(8.).finish())
            .with_child(Shrinkable::new(1., body.finish()).finish())
            .finish();

        let tab_index = row.tab_index;
        let is_active_tab = row.is_active_tab;
        let surface = theme.surface_2();
        list.add_child(
            Hoverable::new(state.row_mouse_state(row.terminal_view_id), move |mouse| {
                let mut container = Container::new(content)
                    .with_padding(Padding::uniform(8.))
                    .with_margin_left(6.)
                    .with_margin_right(6.)
                    .with_corner_radius(CornerRadius::with_all(Radius::Pixels(8.)));
                if is_active_tab || mouse.is_hovered() {
                    container = container.with_background(surface);
                }
                container.finish()
            })
            .with_cursor(Cursor::PointingHand)
            .on_click(move |ctx, _, _| {
                ctx.dispatch_typed_action(WorkspaceAction::ActivateTab(tab_index));
            })
            .finish(),
        );
    }

    let list = ClippedScrollable::vertical(
        state.scroll_state.clone(),
        list.finish(),
        ScrollbarWidth::Auto,
        theme.nonactive_ui_detail().into(),
        theme.active_ui_detail().into(),
        theme.background().into(),
    )
    .finish();

    ConstrainedBox::new(
        Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(header)
            .with_child(Shrinkable::new(1., list).finish())
            .finish(),
    )
    .with_width(PANEL_WIDTH)
    .finish()
}

#[cfg(test)]
#[path = "claude_agents_panel_tests.rs"]
mod tests;
