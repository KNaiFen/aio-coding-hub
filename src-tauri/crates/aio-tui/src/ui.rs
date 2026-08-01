use crate::client::OfflineReason;
use crate::config::{colors_enabled, StatusItem, TuiConfig};
use crate::format::{
    format_cost, format_tokens, now_millis, request_card_lines, scope_label, status_segments,
    truncate_display, truncate_status_line, wrap_status_segments, StatusSegment, StatusTone,
};
use aio_observer_protocol::{CliScope, ObserverRequest, ObserverSnapshotV1};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;
use std::time::Instant;

pub struct LiveState {
    pub scope: CliScope,
    pub snapshot: Option<ObserverSnapshotV1>,
    pub offline: Option<OfflineReason>,
    pub last_success: Option<Instant>,
}

impl LiveState {
    pub fn new(scope: CliScope) -> Self {
        Self {
            scope,
            snapshot: None,
            offline: None,
            last_success: None,
        }
    }

    pub fn apply_snapshot(&mut self, snapshot: ObserverSnapshotV1) {
        self.scope = snapshot.scope;
        self.snapshot = Some(snapshot);
        self.offline = None;
        self.last_success = Some(Instant::now());
    }

    pub fn set_offline(&mut self, reason: OfflineReason) {
        self.offline = Some(reason);
    }

    pub fn stale_label(&self) -> Option<String> {
        let reason = self.offline?;
        let age = self
            .last_success
            .map(|instant| instant.elapsed().as_secs())
            .unwrap_or(0);
        Some(if self.snapshot.is_some() {
            format!("离线 {}s", age)
        } else {
            reason.label().to_string()
        })
    }
}

#[derive(Debug, Clone)]
struct StatuslinePickerRow {
    item: StatusItem,
    enabled: bool,
}

pub struct StatuslinePickerState {
    rows: Vec<StatuslinePickerRow>,
    pub selected: usize,
    pub use_colors: bool,
    notice: Option<String>,
}

impl StatuslinePickerState {
    pub fn new(config: TuiConfig) -> Self {
        let mut rows = config
            .status_items
            .iter()
            .map(|item| StatuslinePickerRow {
                item: *item,
                enabled: true,
            })
            .collect::<Vec<_>>();
        rows.extend(
            StatusItem::ALL
                .into_iter()
                .filter(|item| !config.status_items.contains(item))
                .map(|item| StatuslinePickerRow {
                    item,
                    enabled: false,
                }),
        );
        Self {
            rows,
            selected: 0,
            use_colors: config.use_colors,
            notice: None,
        }
    }

    pub fn selected_items(&self) -> Vec<StatusItem> {
        self.rows
            .iter()
            .filter(|row| row.enabled)
            .map(|row| row.item)
            .collect()
    }

    pub fn config(&self) -> TuiConfig {
        TuiConfig {
            status_items: self.selected_items(),
            use_colors: self.use_colors,
        }
    }

    pub fn move_selection(&mut self, delta: isize) {
        self.selected = self
            .selected
            .saturating_add_signed(delta)
            .min(self.rows.len().saturating_sub(1));
        self.notice = None;
    }

    pub fn move_selected_item(&mut self, delta: isize) {
        let target = self
            .selected
            .saturating_add_signed(delta)
            .min(self.rows.len().saturating_sub(1));
        if target != self.selected {
            self.rows.swap(self.selected, target);
            self.selected = target;
        }
        self.notice = None;
    }

    pub fn toggle_selected(&mut self) {
        let enabled_count = self.rows.iter().filter(|row| row.enabled).count();
        let Some(row) = self.rows.get_mut(self.selected) else {
            return;
        };
        if row.enabled && enabled_count == 1 {
            self.notice = Some("至少保留一个状态栏项目".to_string());
            return;
        }
        row.enabled = !row.enabled;
        self.notice = None;
    }

    pub fn toggle_colors(&mut self) {
        self.use_colors = !self.use_colors;
        self.notice = None;
    }

    pub fn reset(&mut self) {
        *self = Self::new(TuiConfig::default());
        self.notice = Some("已恢复默认项目，按 Enter 保存".to_string());
    }
}

pub struct LogsState {
    pub live: LiveState,
    pub selected: usize,
    pub detail: bool,
    pub detail_scroll: u16,
    pub help: bool,
    pub color: bool,
}

impl LogsState {
    pub fn new(scope: CliScope) -> Self {
        Self {
            live: LiveState::new(scope),
            selected: 0,
            detail: false,
            detail_scroll: 0,
            help: false,
            color: std::env::var_os("NO_COLOR").is_none(),
        }
    }

    pub fn apply_snapshot(&mut self, snapshot: ObserverSnapshotV1) {
        let selected_key = self.selected_request().map(|request| request.key.clone());
        self.live.apply_snapshot(snapshot);
        let count = self.request_count();
        self.selected = selected_key
            .as_deref()
            .and_then(|key| {
                self.requests()
                    .iter()
                    .position(|request| request.key == key)
            })
            .unwrap_or_else(|| self.selected.min(count.saturating_sub(1)));
        if count == 0 {
            self.detail = false;
        }
    }

    pub fn request_count(&self) -> usize {
        self.requests().len()
    }

    pub fn requests(&self) -> Vec<&ObserverRequest> {
        let Some(snapshot) = self.live.snapshot.as_ref() else {
            return Vec::new();
        };
        snapshot
            .active_requests
            .value
            .iter()
            .flat_map(|items| items.iter())
            .chain(
                snapshot
                    .recent_requests
                    .value
                    .iter()
                    .flat_map(|items| items.iter()),
            )
            .collect()
    }

    pub fn selected_request(&self) -> Option<&ObserverRequest> {
        self.requests().get(self.selected).copied()
    }

    pub fn move_selection(&mut self, delta: isize) {
        let count = self.request_count();
        if count == 0 {
            self.selected = 0;
            return;
        }
        self.selected = self
            .selected
            .saturating_add_signed(delta)
            .min(count.saturating_sub(1));
    }

    pub fn set_scope(&mut self, scope: CliScope) {
        self.live = LiveState::new(scope);
        self.selected = 0;
        self.detail = false;
        self.detail_scroll = 0;
    }
}

pub fn draw_status(
    frame: &mut Frame,
    state: &LiveState,
    items: &[StatusItem],
    color: bool,
) {
    let area = frame.area();
    draw_status_in_area(frame, area, state, items, color);
}

fn draw_status_in_area(
    frame: &mut Frame,
    area: Rect,
    state: &LiveState,
    items: &[StatusItem],
    color: bool,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let mut segments = if let Some(snapshot) = state.snapshot.as_ref() {
        status_segments(snapshot, items)
    } else {
        vec![StatusSegment::new(
            state
                .stale_label()
                .unwrap_or_else(|| "AIO 连接中".to_string()),
            if state.offline.is_some() {
                StatusTone::Warning
            } else {
                StatusTone::Default
            },
        )]
    };
    let stale_label = state.snapshot.as_ref().and_then(|_| state.stale_label());
    if let Some(label) = stale_label {
        segments.push(StatusSegment::new(label, StatusTone::Warning));
    }
    let mut lines = wrap_status_segments(&segments, usize::from(area.width));
    if lines.len() > usize::from(area.height) {
        lines.truncate(usize::from(area.height));
        if let Some(last) = lines.last_mut() {
            last.push(StatusSegment::new("…", StatusTone::Separator));
            *last = truncate_status_line(last, usize::from(area.width));
        }
    }
    let height = u16::try_from(lines.len())
        .unwrap_or(area.height)
        .min(area.height);
    let target = Rect {
        x: area.x,
        y: area.y + area.height.saturating_sub(height),
        width: area.width,
        height,
    };
    let lines = lines
        .into_iter()
        .map(|line| {
            Line::from(
                line.into_iter()
                    .map(|segment| {
                        Span::styled(segment.text, status_tone_style(segment.tone, color))
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(lines), target);
}

pub fn draw_statusline_picker(
    frame: &mut Frame,
    state: &mut StatuslinePickerState,
    live: &LiveState,
) {
    let area = frame.area();
    if area.width == 0 || area.height == 0 {
        return;
    }
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(area);
    let effective_colors = colors_enabled(state.use_colors);
    let color_label = if std::env::var_os("NO_COLOR").is_some() {
        "环境禁用"
    } else if state.use_colors {
        "开启"
    } else {
        "关闭"
    };
    let header = [
        truncate_display(
            &format!(
                "状态栏配置 | {} | 已选 {}/{} | 颜色 {}",
                scope_label(live.scope),
                state.selected_items().len(),
                StatusItem::ALL.len(),
                color_label
            ),
            usize::from(chunks[0].width),
        ),
        truncate_display(
            state
                .notice
                .as_deref()
                .unwrap_or("选择项目并查看下方实时预览"),
            usize::from(chunks[0].width),
        ),
    ]
    .join("\n");
    frame.render_widget(
        Paragraph::new(header).style(Style::default().add_modifier(Modifier::BOLD)),
        chunks[0],
    );

    let mut enabled_ordinal = 0_usize;
    let rows = state
        .rows
        .iter()
        .map(|row| {
            if row.enabled {
                enabled_ordinal += 1;
            }
            let checkbox = if row.enabled { "[x]" } else { "[ ]" };
            let order = if row.enabled {
                format!("{enabled_ordinal:02}")
            } else {
                "--".to_string()
            };
            let checkbox_style = if row.enabled && effective_colors {
                Style::default().fg(Color::Green)
            } else {
                muted_style(effective_colors)
            };
            ListItem::new(Line::from(vec![
                Span::styled(checkbox, checkbox_style),
                Span::raw(format!(" {order} {}  ", row.item.label())),
                Span::styled(row.item.key(), muted_style(effective_colors)),
            ]))
        })
        .collect::<Vec<_>>();
    let highlight = if effective_colors {
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().add_modifier(Modifier::REVERSED)
    };
    let mut list_state = ListState::default();
    list_state.select(Some(state.selected));
    frame.render_stateful_widget(
        List::new(rows).highlight_style(highlight),
        chunks[1],
        &mut list_state,
    );

    let preview_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(chunks[2]);
    frame.render_widget(
        Paragraph::new("预览").style(muted_style(effective_colors)),
        preview_chunks[0],
    );
    let selected_items = state.selected_items();
    draw_status_in_area(
        frame,
        preview_chunks[1],
        live,
        &selected_items,
        effective_colors,
    );

    let footer = truncate_display(
        "↑↓选择 ←→排序 Space启用 c颜色 r默认 Enter保存 Esc取消",
        usize::from(chunks[3].width),
    );
    frame.render_widget(
        Paragraph::new(footer).style(muted_style(effective_colors)),
        chunks[3],
    );
}

fn status_tone_style(tone: StatusTone, color: bool) -> Style {
    if tone == StatusTone::Separator {
        return if color {
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::DIM)
        } else {
            Style::default().add_modifier(Modifier::DIM)
        };
    }
    if !color {
        return Style::default();
    }
    let color = match tone {
        StatusTone::Default => return Style::default(),
        StatusTone::Success => Color::Green,
        StatusTone::Warning => Color::Yellow,
        StatusTone::Error => Color::Red,
        StatusTone::Scope => Color::LightBlue,
        StatusTone::Provider => Color::Magenta,
        StatusTone::Model => Color::Cyan,
        StatusTone::Folder => Color::LightMagenta,
        StatusTone::Timing => Color::LightBlue,
        StatusTone::Cost => Color::Green,
        StatusTone::Activity => Color::Yellow,
        StatusTone::Tokens => Color::LightCyan,
        StatusTone::Version => Color::DarkGray,
        StatusTone::Separator => unreachable!(),
    };
    Style::default().fg(color)
}

pub fn draw_logs(frame: &mut Frame, state: &mut LogsState) {
    let area = frame.area();
    if area.width == 0 || area.height == 0 {
        return;
    }
    if state.help {
        draw_help(frame, area, state.color);
        return;
    }
    if state.detail {
        draw_detail(frame, area, state);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(area);
    draw_header(frame, chunks[0], &state.live, state.color);

    let now_ms = now_millis();
    let width = usize::from(chunks[1].width.saturating_sub(1)).max(1);
    let requests = state.requests();
    if requests.is_empty() {
        let message = state
            .live
            .stale_label()
            .unwrap_or_else(|| "暂无请求".to_string());
        frame.render_widget(
            Paragraph::new(message)
                .style(muted_style(state.color))
                .wrap(Wrap { trim: true }),
            chunks[1],
        );
    } else {
        let items = requests
            .iter()
            .map(|request| {
                let lines = request_card_lines(request, now_ms, width)
                    .into_iter()
                    .map(Line::from)
                    .collect::<Vec<_>>();
                let style = request_style(request, state.color);
                ListItem::new(Text::from(lines)).style(style)
            })
            .collect::<Vec<_>>();
        let highlight = if state.color {
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().add_modifier(Modifier::REVERSED)
        };
        let list = List::new(items).highlight_style(highlight);
        let mut list_state = ListState::default();
        list_state.select(Some(state.selected));
        frame.render_stateful_widget(list, chunks[1], &mut list_state);
    }

    let footer = truncate_display(
        "↑↓滚动 Enter详情 Tab切换 ?帮助 q退出",
        usize::from(chunks[2].width),
    );
    frame.render_widget(
        Paragraph::new(footer).style(muted_style(state.color)),
        chunks[2],
    );
}

fn draw_header(frame: &mut Frame, area: Rect, state: &LiveState, color: bool) {
    let online = if state.offline.is_some() {
        state.stale_label().unwrap_or_else(|| "离线".to_string())
    } else if state.snapshot.is_some() {
        "在线".to_string()
    } else {
        "连接中".to_string()
    };
    let concurrency = state
        .snapshot
        .as_ref()
        .map(|snapshot| snapshot.active_inference_count.to_string())
        .unwrap_or_else(|| "—".to_string());
    let first = truncate_display(
        &format!(
            "AIO {} | {} | 并发 {}",
            online,
            scope_label(state.scope),
            concurrency
        ),
        usize::from(area.width),
    );
    let second = state
        .snapshot
        .as_ref()
        .map(|snapshot| {
            let preferred = snapshot
                .preferred_provider
                .value
                .as_ref()
                .map(|provider| provider.provider_name.as_str())
                .unwrap_or("—");
            let today = snapshot.today.value.as_ref();
            let cost = today
                .and_then(|value| value.cost_usd)
                .map(format_cost)
                .unwrap_or_else(|| "—".to_string());
            let tokens = today
                .map(|value| format_tokens(value.total_tokens))
                .unwrap_or_else(|| "—".to_string());
            truncate_display(
                &format!("首选 {} | 今日 {} | {}", preferred, cost, tokens),
                usize::from(area.width),
            )
        })
        .unwrap_or_else(|| "正在读取本地观测接口".to_string());
    let style = if color {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().add_modifier(Modifier::BOLD)
    };
    frame.render_widget(
        Paragraph::new(format!("{first}\n{second}")).style(style),
        area,
    );
}

fn draw_detail(frame: &mut Frame, area: Rect, state: &LogsState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(area);
    frame.render_widget(
        Paragraph::new("请求详情").style(Style::default().add_modifier(Modifier::BOLD)),
        chunks[0],
    );
    let body = state
        .selected_request()
        .map(|request| crate::format::detail_lines(request, now_millis()).join("\n"))
        .unwrap_or_else(|| "请求已不在当前快照中".to_string());
    frame.render_widget(
        Paragraph::new(body)
            .wrap(Wrap { trim: false })
            .scroll((state.detail_scroll, 0)),
        chunks[1],
    );
    let footer = truncate_display("↑↓滚动 Esc返回 r刷新 q退出", usize::from(chunks[2].width));
    frame.render_widget(
        Paragraph::new(footer).style(muted_style(state.color)),
        chunks[2],
    );
}

fn muted_style(color: bool) -> Style {
    if color {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default()
    }
}

fn draw_help(frame: &mut Frame, area: Rect, color: bool) {
    let text = [
        "AIO TUI 操作",
        "",
        "↑/k      上一条",
        "↓/j      下一条",
        "PgUp/PgDn 翻页",
        "Home/End  跳到首尾",
        "Enter      查看详情",
        "Esc        返回列表",
        "Tab        切换 CLI",
        "r          立即刷新",
        "?          关闭帮助",
        "q/Ctrl-C   退出",
    ]
    .join("\n");
    let style = if color {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };
    frame.render_widget(
        Paragraph::new(text).style(style).wrap(Wrap { trim: false }),
        area,
    );
}

fn request_style(request: &ObserverRequest, color: bool) -> Style {
    if !color {
        return Style::default();
    }
    let color = if request.state == aio_observer_protocol::ObserverRequestState::Active {
        Color::Cyan
    } else if request.interrupted {
        Color::Yellow
    } else if request
        .status
        .is_some_and(|status| (200..300).contains(&status))
    {
        Color::Green
    } else {
        Color::Red
    };
    Style::default().fg(color)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aio_observer_protocol::{
        ObserverGatewayStatus, ObserverRequestState, ObserverSection, ObserverTodayUsage,
        OBSERVER_PROTOCOL_VERSION,
    };
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn rendered_non_space_symbols(terminal: &Terminal<TestBackend>) -> String {
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .filter(|symbol| !symbol.trim().is_empty())
            .collect()
    }

    fn empty_snapshot(scope: CliScope) -> ObserverSnapshotV1 {
        ObserverSnapshotV1 {
            protocol_version: OBSERVER_PROTOCOL_VERSION,
            app_version: "0.60.39".to_string(),
            generated_at_ms: 1,
            scope,
            gateway: ObserverGatewayStatus {
                running: true,
                port: Some(37123),
            },
            preferred_provider: ObserverSection::empty(),
            last_request: ObserverSection::empty(),
            dominant_provider: ObserverSection::empty(),
            active_inference_count: 13,
            today: ObserverSection::ready(ObserverTodayUsage {
                total_tokens: 507_900_000,
                cost_usd: Some(12.34),
            }),
            active_requests: ObserverSection::ready(Vec::new()),
            recent_requests: ObserverSection::ready(Vec::new()),
        }
    }

    fn terminal_request(key: &str) -> ObserverRequest {
        ObserverRequest {
            key: key.to_string(),
            state: ObserverRequestState::Terminal,
            cli_key: "codex".to_string(),
            method: "POST".to_string(),
            path: "/v1/responses".to_string(),
            model: Some("gpt-5".to_string()),
            provider_name: Some("Provider".to_string()),
            status: Some(200),
            error_code: None,
            interrupted: false,
            created_at_ms: 1,
            last_activity_ms: 2,
            duration_ms: Some(1),
            ttfb_ms: Some(1),
            attempt_count: 1,
            retry_count: 0,
            provider_switch_count: 0,
            has_failover: false,
            session_reuse: false,
            session_id: None,
            folder_name: None,
            usage: None,
            cost_usd: None,
            route: Vec::new(),
            context_compaction: None,
        }
    }

    #[test]
    fn narrow_logs_layout_renders_without_overflow() {
        let backend = TestBackend::new(32, 12);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let mut state = LogsState::new(CliScope::Codex);
        state.apply_snapshot(empty_snapshot(CliScope::Codex));
        terminal
            .draw(|frame| draw_logs(frame, &mut state))
            .expect("draw");
        // TestBackend stores a padding cell after each wide glyph. Ignore
        // whitespace-only cells when checking the rendered CJK text.
        let text = rendered_non_space_symbols(&terminal);
        assert!(text.contains("并发13"));
        assert!(text.contains("暂无请求"));
    }

    #[test]
    fn one_row_status_keeps_a_visible_prefix() {
        let backend = TestBackend::new(24, 1);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let mut state = LiveState::new(CliScope::Codex);
        state.apply_snapshot(empty_snapshot(CliScope::Codex));
        terminal
            .draw(|frame| {
                draw_status(
                    frame,
                    &state,
                    &StatusItem::DEFAULT,
                    true,
                )
            })
            .expect("draw");
        let text = rendered_non_space_symbols(&terminal);
        assert!(text.contains("首选"));
    }

    #[test]
    fn statusline_picker_reorders_enabled_items_and_keeps_one_selected() {
        let mut picker = StatuslinePickerState::new(TuiConfig::default());
        assert_eq!(picker.selected_items(), StatusItem::DEFAULT.to_vec());

        picker.move_selected_item(1);
        assert_eq!(
            picker.selected_items()[..2],
            [StatusItem::LastRequest, StatusItem::PreferredProvider]
        );

        let mut singleton = StatuslinePickerState::new(TuiConfig {
            status_items: vec![StatusItem::Gateway],
            use_colors: true,
        });
        singleton.toggle_selected();
        assert_eq!(singleton.selected_items(), vec![StatusItem::Gateway]);
        assert_eq!(
            singleton.notice.as_deref(),
            Some("至少保留一个状态栏项目")
        );
    }

    #[test]
    fn statusline_picker_renders_in_a_narrow_terminal() {
        let backend = TestBackend::new(30, 12);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let mut picker = StatuslinePickerState::new(TuiConfig::default());
        let mut live = LiveState::new(CliScope::Codex);
        live.apply_snapshot(empty_snapshot(CliScope::Codex));
        terminal
            .draw(|frame| draw_statusline_picker(frame, &mut picker, &live))
            .expect("draw");
        let text = rendered_non_space_symbols(&terminal);
        assert!(text.contains("状态栏配置"));
        assert!(text.contains("预览"));
    }

    #[test]
    fn refresh_preserves_selection_by_request_key() {
        let mut state = LogsState::new(CliScope::Codex);
        let mut initial = empty_snapshot(CliScope::Codex);
        initial.recent_requests = ObserverSection::ready(vec![
            terminal_request("newest"),
            terminal_request("selected"),
        ]);
        state.apply_snapshot(initial);
        state.selected = 1;

        let mut refreshed = empty_snapshot(CliScope::Codex);
        refreshed.active_requests = ObserverSection::ready(vec![terminal_request("active")]);
        refreshed.recent_requests = ObserverSection::ready(vec![
            terminal_request("newest"),
            terminal_request("selected"),
        ]);
        state.apply_snapshot(refreshed);

        assert_eq!(
            state.selected_request().map(|request| request.key.as_str()),
            Some("selected")
        );
    }
}
