use crate::client::OfflineReason;
use crate::config::{colors_enabled, StatusItem, TuiConfig};
use crate::format::{
    cli_label, format_cost, format_duration, format_tokens, now_millis, request_card_lines,
    scope_label, status_segments, truncate_display, truncate_status_line, wrap_status_segments,
    RequestCardLineKind, StatusSegment, StatusTone,
};
use crate::palette::{Palette, Tone};
use aio_observer_protocol::{
    CliScope, ObserverProviderAvailabilityTestResult, ObserverProviderStatus, ObserverRequest,
    ObserverSnapshotV1,
};
use chrono::{DateTime, Local, Utc};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
#[cfg(test)]
use ratatui::style::Color;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

const SELECTION_IDLE_TIMEOUT: Duration = Duration::from_secs(5);
const STATUSLINE_GUTTER_COLUMNS: u16 = 2;

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
            if reason == OfflineReason::Busy {
                format!("观测繁忙 {}s", age)
            } else {
                format!("离线 {}s", age)
            }
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
    pub selected: Option<usize>,
    pub provider_selected: Option<usize>,
    request_selection_expires_at: Option<Instant>,
    provider_selection_expires_at: Option<Instant>,
    provider_key: Option<i64>,
    pub view: DashboardView,
    pub providers_pending: bool,
    pub detail: bool,
    pub detail_scroll: u16,
    pub help: bool,
    pub color: bool,
    provider_probes: HashMap<i64, ProviderProbeState>,
}

#[derive(Clone, PartialEq, Eq)]
pub enum ProviderProbeState {
    Running,
    Complete(ObserverProviderAvailabilityTestResult),
    Failed(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashboardView {
    Requests,
    Providers,
}

impl LogsState {
    pub fn new(scope: CliScope) -> Self {
        Self {
            live: LiveState::new(scope),
            selected: None,
            provider_selected: None,
            request_selection_expires_at: None,
            provider_selection_expires_at: None,
            provider_key: None,
            view: DashboardView::Requests,
            providers_pending: false,
            detail: false,
            detail_scroll: 0,
            help: false,
            color: std::env::var_os("NO_COLOR").is_none(),
            provider_probes: HashMap::new(),
        }
    }

    pub fn apply_snapshot(&mut self, snapshot: ObserverSnapshotV1) {
        let selected_key = self.selected_request().map(|request| request.key.clone());
        let selected_provider_id = self
            .selected_provider()
            .map(|provider| provider.provider_id)
            .or(self.provider_key);
        self.live.apply_snapshot(snapshot);
        self.selected = selected_key.as_deref().and_then(|key| {
            self.requests()
                .iter()
                .position(|request| request.key == key)
        });
        if self.selected.is_none() {
            self.request_selection_expires_at = None;
        }
        if self.view == DashboardView::Providers {
            self.providers_pending = false;
        }
        if self
            .live
            .snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.providers.is_some())
        {
            self.provider_selected = selected_provider_id.and_then(|provider_id| {
                self.providers()
                    .iter()
                    .position(|provider| provider.provider_id == provider_id)
            });
            self.provider_key = self
                .selected_provider()
                .map(|provider| provider.provider_id);
            if self.provider_selected.is_none() {
                self.provider_selection_expires_at = None;
            }
        }
        if self.detail && !self.has_selected_item() {
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
        self.selected
            .and_then(|selected| self.requests().get(selected).copied())
    }

    pub fn provider_count(&self) -> usize {
        self.providers().len()
    }

    pub fn providers(&self) -> Vec<&ObserverProviderStatus> {
        self.live
            .snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.providers.as_ref())
            .and_then(|section| section.value.as_ref())
            .map(|collection| collection.items.iter().collect())
            .unwrap_or_default()
    }

    pub fn selected_provider(&self) -> Option<&ObserverProviderStatus> {
        self.provider_selected
            .and_then(|selected| self.providers().get(selected).copied())
    }

    pub fn move_selection(&mut self, delta: isize, now: Instant) {
        let count = self.current_count();
        if count == 0 {
            self.clear_current_selection();
            return;
        }
        let selected = self
            .current_selection()
            .unwrap_or(0)
            .saturating_add_signed(delta)
            .min(count.saturating_sub(1));
        self.select_current(selected, now);
    }

    pub fn current_count(&self) -> usize {
        match self.view {
            DashboardView::Requests => self.request_count(),
            DashboardView::Providers => self.provider_count(),
        }
    }

    pub fn current_selection(&self) -> Option<usize> {
        match self.view {
            DashboardView::Requests => self.selected,
            DashboardView::Providers => self.provider_selected,
        }
    }

    pub fn select_current(&mut self, selected: usize, now: Instant) {
        let count = self.current_count();
        if count == 0 {
            self.clear_current_selection();
            return;
        }
        let selected = selected.min(count.saturating_sub(1));
        let expires_at = now + SELECTION_IDLE_TIMEOUT;
        match self.view {
            DashboardView::Requests => {
                self.selected = Some(selected);
                self.request_selection_expires_at = Some(expires_at);
            }
            DashboardView::Providers => {
                self.provider_selected = Some(selected);
                self.provider_selection_expires_at = Some(expires_at);
                self.provider_key = self
                    .selected_provider()
                    .map(|provider| provider.provider_id);
            }
        }
    }

    pub fn clear_current_selection(&mut self) {
        match self.view {
            DashboardView::Requests => {
                self.selected = None;
                self.request_selection_expires_at = None;
            }
            DashboardView::Providers => {
                self.provider_selected = None;
                self.provider_selection_expires_at = None;
                self.provider_key = None;
            }
        }
    }

    pub fn suspend_current_selection_expiry(&mut self) {
        match self.view {
            DashboardView::Requests => self.request_selection_expires_at = None,
            DashboardView::Providers => self.provider_selection_expires_at = None,
        }
    }

    pub fn resume_current_selection_expiry(&mut self, now: Instant) {
        if !self.has_selected_item() {
            return;
        }
        let expires_at = Some(now + SELECTION_IDLE_TIMEOUT);
        match self.view {
            DashboardView::Requests => self.request_selection_expires_at = expires_at,
            DashboardView::Providers => self.provider_selection_expires_at = expires_at,
        }
    }

    pub fn expire_inactive_selections(&mut self, now: Instant) -> bool {
        let request_expired = self
            .request_selection_expires_at
            .is_some_and(|expires_at| now >= expires_at);
        let provider_expired = self
            .provider_selection_expires_at
            .is_some_and(|expires_at| now >= expires_at);
        if request_expired {
            self.selected = None;
            self.request_selection_expires_at = None;
        }
        if provider_expired {
            self.provider_selected = None;
            self.provider_selection_expires_at = None;
            self.provider_key = None;
        }
        request_expired || provider_expired
    }

    pub fn has_selected_item(&self) -> bool {
        match self.view {
            DashboardView::Requests => self.selected_request().is_some(),
            DashboardView::Providers => self.selected_provider().is_some(),
        }
    }

    pub fn switch_view(&mut self, view: DashboardView) {
        if self.view == view {
            return;
        }
        self.view = view;
        self.detail = false;
        self.detail_scroll = 0;
        self.providers_pending = view == DashboardView::Providers
            && self
                .live
                .snapshot
                .as_ref()
                .is_none_or(|snapshot| snapshot.providers.is_none());
    }

    pub fn set_scope(&mut self, scope: CliScope) {
        self.live = LiveState::new(scope);
        self.selected = None;
        self.provider_selected = None;
        self.request_selection_expires_at = None;
        self.provider_selection_expires_at = None;
        self.provider_key = None;
        self.providers_pending = self.view == DashboardView::Providers;
        self.detail = false;
        self.detail_scroll = 0;
    }

    pub fn begin_provider_probe(&mut self) -> Option<i64> {
        if self.view != DashboardView::Providers || !self.detail {
            return None;
        }
        let provider_id = self.selected_provider()?.provider_id;
        if matches!(
            self.provider_probes.get(&provider_id),
            Some(ProviderProbeState::Running)
        ) {
            return None;
        }
        self.provider_probes
            .insert(provider_id, ProviderProbeState::Running);
        Some(provider_id)
    }

    pub fn finish_provider_probe(
        &mut self,
        provider_id: i64,
        result: Result<ObserverProviderAvailabilityTestResult, OfflineReason>,
    ) {
        if !matches!(
            self.provider_probes.get(&provider_id),
            Some(ProviderProbeState::Running)
        ) {
            return;
        }
        let state = match result {
            Ok(result) => ProviderProbeState::Complete(result),
            Err(reason) => ProviderProbeState::Failed(reason.label().to_string()),
        };
        self.provider_probes.insert(provider_id, state);
    }
}

pub fn draw_status(frame: &mut Frame, state: &LiveState, items: &[StatusItem], color: bool) {
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
    if area.width <= STATUSLINE_GUTTER_COLUMNS || area.height == 0 {
        return;
    }
    let content_area = Rect {
        x: area.x.saturating_add(STATUSLINE_GUTTER_COLUMNS),
        y: area.y,
        width: area.width.saturating_sub(STATUSLINE_GUTTER_COLUMNS),
        height: area.height,
    };
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
    let mut lines = wrap_status_segments(&segments, usize::from(content_area.width));
    if lines.len() > usize::from(content_area.height) {
        lines.truncate(usize::from(content_area.height));
        if let Some(last) = lines.last_mut() {
            last.push(StatusSegment::new("…", StatusTone::Separator));
            *last = truncate_status_line(last, usize::from(content_area.width));
        }
    }
    let height = u16::try_from(lines.len())
        .unwrap_or(content_area.height)
        .min(content_area.height);
    let target = Rect {
        x: content_area.x,
        y: content_area.y + content_area.height.saturating_sub(height),
        width: content_area.width,
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
        Paragraph::new(header).style(
            Palette::detected(effective_colors)
                .style(Tone::Accent)
                .add_modifier(Modifier::BOLD),
        ),
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
            let checkbox_style = if row.enabled {
                Palette::detected(effective_colors).style(Tone::Success)
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
    let highlight = Palette::detected(effective_colors)
        .selected(Style::default())
        .add_modifier(Modifier::BOLD);
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
    let tone = match tone {
        StatusTone::Default => Tone::Default,
        StatusTone::Success => Tone::Success,
        StatusTone::Warning => Tone::Warning,
        StatusTone::Error => Tone::Error,
        StatusTone::Scope | StatusTone::Activity | StatusTone::Version => Tone::Accent,
        StatusTone::Provider => Tone::Provider,
        StatusTone::Model => Tone::Info,
        StatusTone::Folder | StatusTone::Timing | StatusTone::Cost | StatusTone::Tokens => {
            Tone::Success
        }
        StatusTone::Separator => Tone::Muted,
    };
    Palette::detected(color).style(tone)
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
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(area);
    draw_header(frame, chunks[0], &state.live, state.color);
    draw_header_separator(frame, chunks[1], state.color);

    let width = usize::from(chunks[2].width.saturating_sub(1)).max(1);
    match state.view {
        DashboardView::Requests => draw_request_list(frame, chunks[2], state, width),
        DashboardView::Providers => draw_provider_list(frame, chunks[2], state, width),
    }

    let footer = truncate_display(
        "←→视图 ↑↓滚动 Enter详情 Tab切CLI ?帮助 q退出",
        usize::from(chunks[3].width),
    );
    frame.render_widget(
        Paragraph::new(footer).style(muted_style(state.color)),
        chunks[3],
    );
}

fn draw_header_separator(frame: &mut Frame, area: Rect, color: bool) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    frame.render_widget(
        Paragraph::new("─".repeat(usize::from(area.width))).style(item_separator_style(color)),
        area,
    );
}

fn draw_request_list(frame: &mut Frame, area: Rect, state: &mut LogsState, width: usize) {
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
            area,
        );
        return;
    }
    let now_ms = now_millis();
    let items = requests
        .iter()
        .enumerate()
        .map(|(index, request)| {
            let selected = state.selected == Some(index);
            let mut lines = request_card_lines(request, now_ms, width)
                .into_iter()
                .map(|line| {
                    Line::styled(
                        line.text,
                        request_line_style(request, line.kind, state.color, selected),
                    )
                })
                .collect::<Vec<_>>();
            lines.push(Line::styled(
                "─".repeat(width),
                item_separator_style(state.color),
            ));
            ListItem::new(lines)
        })
        .collect::<Vec<_>>();
    let mut list_state = ListState::default();
    list_state.select(state.selected);
    frame.render_stateful_widget(List::new(items), area, &mut list_state);
}

fn draw_provider_list(frame: &mut Frame, area: Rect, state: &mut LogsState, width: usize) {
    let providers = state.providers();
    if providers.is_empty() {
        let message = provider_empty_message(state);
        frame.render_widget(
            Paragraph::new(message)
                .style(muted_style(state.color))
                .wrap(Wrap { trim: true }),
            area,
        );
        return;
    }
    let items = providers
        .iter()
        .enumerate()
        .map(|(index, provider)| {
            let selected = state.provider_selected == Some(index);
            let mut lines = provider_card_lines(provider, width, state.color, selected);
            lines.push(Line::styled(
                "─".repeat(width),
                item_separator_style(state.color),
            ));
            ListItem::new(lines)
        })
        .collect::<Vec<_>>();
    let mut list_state = ListState::default();
    list_state.select(state.provider_selected);
    frame.render_stateful_widget(List::new(items), area, &mut list_state);
}

fn provider_empty_message(state: &LogsState) -> String {
    if let Some(label) = state.live.stale_label() {
        return label;
    }
    if state.providers_pending {
        return "正在读取供应商状态".to_string();
    }
    match state
        .live
        .snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.providers.as_ref())
    {
        None => "当前 AIO 版本不支持供应商视图".to_string(),
        Some(section) if !section.available => "供应商状态暂不可用".to_string(),
        Some(_) => "暂无供应商".to_string(),
    }
}

fn dashboard_header_lines(state: &LiveState, width: usize) -> [String; 2] {
    let concurrency = state
        .snapshot
        .as_ref()
        .map(|snapshot| snapshot.active_inference_count.to_string())
        .unwrap_or_else(|| "—".to_string());
    let preferred = state
        .snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.preferred_provider.value.as_ref())
        .map(|provider| provider.provider_name.as_str())
        .unwrap_or("—");
    let today = state
        .snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.today.value.as_ref());
    let cost = today
        .and_then(|value| value.cost_usd)
        .map(format_cost)
        .unwrap_or_else(|| "—".to_string());
    let tokens = today
        .map(|value| format_tokens(value.total_tokens))
        .unwrap_or_else(|| "—".to_string());

    [
        truncate_display(&format!("并发 {concurrency} | 首选 {preferred}"), width),
        truncate_display(
            &format!("{} | 今日 {cost} | {tokens}", scope_label(state.scope)),
            width,
        ),
    ]
}

fn draw_header(frame: &mut Frame, area: Rect, state: &LiveState, color: bool) {
    let [first, second] = dashboard_header_lines(state, usize::from(area.width));
    let style = Palette::detected(color)
        .style(Tone::Accent)
        .add_modifier(Modifier::BOLD);
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
        Paragraph::new(match state.view {
            DashboardView::Requests => "请求详情",
            DashboardView::Providers => "供应商详情",
        })
        .style(
            Palette::detected(state.color)
                .style(Tone::Accent)
                .add_modifier(Modifier::BOLD),
        ),
        chunks[0],
    );
    let body = match state.view {
        DashboardView::Requests => state
            .selected_request()
            .map(|request| {
                styled_detail_lines(
                    crate::format::detail_lines(request, now_millis()),
                    state.color,
                )
            })
            .unwrap_or_else(|| {
                vec![Line::styled(
                    "请求已不在当前快照中",
                    muted_style(state.color),
                )]
            }),
        DashboardView::Providers => state
            .selected_provider()
            .map(|provider| {
                styled_detail_lines(
                    provider_detail_lines(
                        provider,
                        state.provider_probes.get(&provider.provider_id),
                    ),
                    state.color,
                )
            })
            .unwrap_or_else(|| {
                vec![Line::styled(
                    "供应商已不在当前快照中",
                    muted_style(state.color),
                )]
            }),
    };
    frame.render_widget(
        Paragraph::new(body)
            .wrap(Wrap { trim: false })
            .scroll((state.detail_scroll, 0)),
        chunks[1],
    );
    let footer = truncate_display(
        match state.view {
            DashboardView::Requests => "↑↓滚动 Esc返回 r刷新 q退出",
            DashboardView::Providers => "↑↓滚动 t测试 Esc返回 r刷新 q退出",
        },
        usize::from(chunks[2].width),
    );
    frame.render_widget(
        Paragraph::new(footer).style(muted_style(state.color)),
        chunks[2],
    );
}

fn styled_detail_lines(lines: Vec<String>, color: bool) -> Vec<Line<'static>> {
    const SECTION_TITLES: [&str; 4] = ["上下文压缩", "路由链", "短期可用性", "手动可用性测试"];
    let palette = Palette::detected(color);
    lines
        .into_iter()
        .map(|line| {
            if line.is_empty() {
                return Line::default();
            }
            if SECTION_TITLES.contains(&line.as_str()) {
                return Line::styled(
                    line,
                    palette.style(Tone::Accent).add_modifier(Modifier::BOLD),
                );
            }
            if let Some((label, value)) = line.split_once("  ") {
                return Line::from(vec![
                    Span::styled(label.to_string(), palette.style(Tone::Accent)),
                    Span::raw("  "),
                    Span::raw(value.to_string()),
                ]);
            }
            if let Some((label, value)) = line.split_once('：') {
                return Line::from(vec![
                    Span::styled(format!("{label}："), palette.style(Tone::Accent)),
                    Span::raw(value.to_string()),
                ]);
            }
            Line::raw(line)
        })
        .collect()
}

fn provider_detail_lines(
    provider: &ObserverProviderStatus,
    probe: Option<&ProviderProbeState>,
) -> Vec<String> {
    let route = match provider.route_rank {
        Some(rank) if provider.route_enabled => format!("第 {rank} 位（启用）"),
        Some(rank) => format!("第 {rank} 位（停用）"),
        None => "不在当前路由".to_string(),
    };
    let circuit = match (
        provider.circuit_state.as_deref(),
        provider.circuit_failure_count,
        provider.circuit_failure_threshold,
    ) {
        (Some(state), Some(count), Some(threshold)) => {
            format!("{state}，失败 {count}/{threshold}")
        }
        _ => "不可用".to_string(),
    };
    let mut lines = vec![
        format!("名称：{}", provider.provider_name),
        format!("CLI：{}", cli_label(&provider.cli_key)),
        format!("路由：{route}"),
        format!(
            "供应商：{}",
            if provider.provider_enabled {
                "启用"
            } else {
                "停用"
            }
        ),
        format!(
            "认证：{}",
            if provider.auth_kind == "oauth" {
                "OAuth"
            } else {
                "API Key"
            }
        ),
        format!(
            "资格：{}{}",
            provider_eligibility_label(&provider.eligibility),
            if provider.preferred {
                "（首选）"
            } else {
                ""
            }
        ),
        format!("熔断：{circuit}"),
    ];
    if let Some(recover_at) = provider.recover_at_unix {
        lines.push(format!("预计恢复：{}", format_reset_at(recover_at)));
    }
    if provider.spend_windows.is_empty() {
        lines.push("消费限额：未配置".to_string());
    } else {
        lines.push("消费限额：".to_string());
        lines.extend(provider.spend_windows.iter().map(|window| {
            format!(
                "  {}：{} / {}",
                spend_window_label(&window.window),
                format_cost(window.usage_usd),
                format_cost(window.limit_usd)
            )
        }));
    }
    if let Some(usage) = provider.account_usage.as_ref() {
        let balance = match (usage.state.as_str(), usage.amount) {
            ("available", Some(amount)) => {
                format_provider_account_amount(amount, usage.unit.as_deref())
            }
            ("available", None) => "—".to_string(),
            ("loading", _) => "获取中".to_string(),
            _ => "获取失败".to_string(),
        };
        lines.push(format!("账户余额：{balance}"));
    }
    if let Some(quota) = displayable_oauth_quota(provider) {
        if let Some(label) = non_empty(quota.short_label.as_deref()) {
            lines.push(format!("OAuth 标签：{label}"));
        }
        if let Some(value) = non_empty(quota.five_hour_text.as_deref()) {
            lines.push(format!("OAuth 5h：{value}"));
        }
        if let Some(value) = non_empty(quota.weekly_text.as_deref()) {
            lines.push(format!("OAuth 周：{value}"));
        }
        if let Some(reset_at) = quota.five_hour_reset_at_unix {
            lines.push(format!("5h 重置：{}", format_reset_at(reset_at)));
        }
        if let Some(reset_at) = quota.weekly_reset_at_unix {
            lines.push(format!("周重置：{}", format_reset_at(reset_at)));
        }
    }
    if let Some(availability) = provider.availability.as_ref() {
        lines.extend(provider_availability_detail_lines(availability));
    }
    if let Some(probe) = probe {
        lines.extend(provider_probe_detail_lines(probe));
    }
    lines
}

fn provider_probe_detail_lines(probe: &ProviderProbeState) -> Vec<String> {
    let mut lines = vec![String::new(), "手动可用性测试".to_string()];
    match probe {
        ProviderProbeState::Running => lines.push("结果：测试中".to_string()),
        ProviderProbeState::Failed(message) => {
            lines.push("结果：测试失败".to_string());
            lines.push(format!("原因：{message}"));
        }
        ProviderProbeState::Complete(result) => {
            lines.push(format!(
                "结果：{}",
                if result.ok { "可用" } else { "不可用" }
            ));
            lines.push(format!(
                "HTTP：{}",
                result
                    .status
                    .map(|status| status.to_string())
                    .unwrap_or_else(|| "—".to_string())
            ));
            lines.push(format!("耗时：{}", format_duration(result.latency_ms)));
            if !result.base_url.is_empty() {
                lines.push(format!("Base URL：{}", result.base_url));
            }
            if let Some(error) = non_empty(result.error.as_deref()) {
                lines.push(format!("原因：{error}"));
            }
            if let Some(preview) = non_empty(result.response_preview.as_deref()) {
                lines.push("响应预览：".to_string());
                lines.extend(preview.lines().map(|line| format!("  {line}")));
            }
        }
    }
    lines
}

fn provider_availability_detail_lines(
    availability: &aio_observer_protocol::ObserverProviderAvailabilityTimeline,
) -> Vec<String> {
    let mut lines = vec![
        String::new(),
        "短期可用性".to_string(),
        format!(
            "范围：{}h，每格 {} 分钟",
            availability.hours, availability.bucket_minutes
        ),
        format!(
            "汇总：成功 {}，失败 {}",
            availability.success_count, availability.failure_count
        ),
    ];
    lines.extend(availability.buckets.iter().take(12).map(|bucket| {
        let state = match bucket.state {
            aio_observer_protocol::ObserverProviderAvailabilityState::Healthy => "可用",
            aio_observer_protocol::ObserverProviderAvailabilityState::Unhealthy => "不可用",
            aio_observer_protocol::ObserverProviderAvailabilityState::NoData => "无数据",
        };
        format!(
            "  {}-{}  {state}  成{} 败{}",
            format_local_minute(bucket.start_at_ms),
            format_local_minute(bucket.end_at_ms),
            bucket.success_count,
            bucket.failure_count
        )
    }));
    lines
}

fn format_minute_with_offset(timestamp_ms: i64, offset_seconds: i32) -> String {
    let local_ms = timestamp_ms.saturating_add(i64::from(offset_seconds).saturating_mul(1_000));
    let minutes = local_ms.div_euclid(60_000).rem_euclid(24 * 60);
    format!("{:02}:{:02}", minutes / 60, minutes % 60)
}

fn format_local_minute(timestamp_ms: i64) -> String {
    let Some(timestamp) = DateTime::<Utc>::from_timestamp_millis(timestamp_ms) else {
        return "—".to_string();
    };
    format_minute_with_offset(
        timestamp_ms,
        timestamp.with_timezone(&Local).offset().local_minus_utc(),
    )
}

fn format_reset_at(reset_at_unix: i64) -> String {
    let now_unix = now_millis() / 1000;
    if reset_at_unix <= now_unix {
        return "已到期".to_string();
    }
    format!(
        "{}后",
        format_duration(reset_at_unix.saturating_sub(now_unix).saturating_mul(1000))
    )
}

fn muted_style(color: bool) -> Style {
    Palette::detected(color).style(Tone::Muted)
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
        "←/→        请求/供应商视图",
        "Tab        切换 CLI",
        "t          测试当前供应商",
        "r          立即刷新",
        "?          关闭帮助",
        "q/Ctrl-C   退出",
    ]
    .join("\n");
    let style = Palette::detected(color).style(Tone::Accent);
    frame.render_widget(
        Paragraph::new(text).style(style).wrap(Wrap { trim: false }),
        area,
    );
}

fn request_line_style(
    request: &ObserverRequest,
    kind: RequestCardLineKind,
    color: bool,
    selected: bool,
) -> Style {
    let tone = match kind {
        RequestCardLineKind::Status
            if request.state == aio_observer_protocol::ObserverRequestState::Active =>
        {
            Tone::Accent
        }
        RequestCardLineKind::Status if request.interrupted => Tone::Warning,
        RequestCardLineKind::Status
            if request
                .status
                .is_some_and(|status| (200..300).contains(&status)) =>
        {
            Tone::Success
        }
        RequestCardLineKind::Status => Tone::Error,
        RequestCardLineKind::Model | RequestCardLineKind::ModelTarget => Tone::Info,
        RequestCardLineKind::Provider => Tone::Provider,
        RequestCardLineKind::Route
            if request.provider_switch_count > 0 || request.retry_count > 0 =>
        {
            Tone::Warning
        }
        RequestCardLineKind::Route => Tone::Success,
        RequestCardLineKind::Metrics => Tone::Accent,
    };
    let mut style = Palette::detected(color).style(tone);
    if selected {
        style = Palette::detected(color).selected(style);
    }
    style
}

#[derive(Clone, Copy)]
enum ProviderTone {
    Default,
    Label,
    Accent,
    Success,
    Warning,
    Error,
    Muted,
}

fn provider_card_lines(
    provider: &ObserverProviderStatus,
    width: usize,
    color: bool,
    selected: bool,
) -> Vec<Line<'static>> {
    let rank = provider
        .route_rank
        .map(|rank| format!("#{rank}"))
        .unwrap_or_else(|| "--".to_string());
    let provider_enabled = if provider.provider_enabled {
        ("开", ProviderTone::Success)
    } else {
        ("停", ProviderTone::Error)
    };
    let route_enabled = if provider.route_rank.is_none() {
        ("外", ProviderTone::Muted)
    } else if provider.route_enabled {
        ("开", ProviderTone::Success)
    } else {
        ("停", ProviderTone::Error)
    };
    let auth = if provider.auth_kind == "oauth" {
        "OAuth"
    } else {
        "API Key"
    };
    let circuit = match (
        provider.circuit_state.as_deref(),
        provider.circuit_failure_count,
        provider.circuit_failure_threshold,
    ) {
        (Some(state), Some(count), Some(threshold)) => {
            (state.to_string(), format!(" {count}/{threshold}"))
        }
        (Some(state), _, _) => (state.to_string(), String::new()),
        _ => ("—".to_string(), String::new()),
    };
    let eligibility_tone = provider_eligibility_tone(&provider.eligibility);
    let circuit_tone = circuit_tone(&circuit.0);
    let circuit_label_tone = if matches!(circuit_tone, ProviderTone::Error) {
        ProviderTone::Error
    } else {
        ProviderTone::Label
    };
    let spend_tone = if provider.eligibility == "spend_limited" {
        ProviderTone::Warning
    } else {
        ProviderTone::Success
    };

    let mut lines = vec![
        provider_line(
            vec![
                provider_span(rank, ProviderTone::Muted, color),
                provider_span(" ", ProviderTone::Default, color),
                provider_span(provider.provider_name.clone(), ProviderTone::Accent, color),
                provider_span(
                    if provider.preferred { " [首选]" } else { "" },
                    ProviderTone::Success,
                    color,
                ),
            ],
            width,
            color,
            selected,
        ),
        provider_line(
            vec![
                provider_span(
                    cli_label(&provider.cli_key).to_string(),
                    ProviderTone::Accent,
                    color,
                ),
                provider_separator(color),
                provider_span(auth, ProviderTone::Default, color),
                provider_separator(color),
                provider_span("供", ProviderTone::Label, color),
                provider_span(provider_enabled.0, provider_enabled.1, color),
                provider_span(" ", ProviderTone::Default, color),
                provider_span("路", ProviderTone::Label, color),
                provider_span(route_enabled.0, route_enabled.1, color),
            ],
            width,
            color,
            selected,
        ),
        provider_line(
            vec![
                provider_span(
                    provider_eligibility_label(&provider.eligibility),
                    eligibility_tone,
                    color,
                ),
                provider_separator(color),
                provider_span("熔", circuit_label_tone, color),
                provider_span(" ", ProviderTone::Default, color),
                provider_span(circuit.0, circuit_tone, color),
                provider_span(circuit.1, circuit_tone, color),
            ],
            width,
            color,
            selected,
        ),
    ];

    if !provider.spend_windows.is_empty() {
        lines.push(provider_spend_line(
            provider, width, color, selected, spend_tone,
        ));
    }

    if let Some(usage) = provider.account_usage.as_ref() {
        lines.push(provider_account_usage_line(usage, width, color, selected));
    }

    if let Some(quota) = displayable_oauth_quota(provider) {
        let oauth_tone = if provider.eligibility == "oauth_limited" {
            ProviderTone::Warning
        } else {
            ProviderTone::Success
        };
        let mut spans = vec![
            provider_span("OAuth", ProviderTone::Label, color),
            provider_span(" ", ProviderTone::Default, color),
        ];
        let mut values = Vec::new();
        if let Some(label) = non_empty(quota.short_label.as_deref()) {
            values.push(label.to_string());
        }
        if let Some(value) = non_empty(quota.five_hour_text.as_deref()) {
            values.push(format!("5h {value}"));
        }
        if let Some(value) = non_empty(quota.weekly_text.as_deref()) {
            values.push(format!("周 {value}"));
        }
        for (index, value) in values.into_iter().enumerate() {
            if index > 0 {
                spans.push(provider_separator(color));
            }
            spans.push(provider_span(value, oauth_tone, color));
        }
        lines.push(provider_line(spans, width, color, selected));
    }

    if let Some(availability) = provider.availability.as_ref() {
        lines.push(provider_availability_line(
            availability,
            width,
            color,
            selected,
        ));
    }

    lines
}

fn provider_spend_line(
    provider: &ObserverProviderStatus,
    width: usize,
    color: bool,
    selected: bool,
    value_tone: ProviderTone,
) -> Line<'static> {
    let mut spans = vec![provider_span("限", ProviderTone::Label, color)];
    for window in &provider.spend_windows {
        spans.push(provider_span(" ", ProviderTone::Default, color));
        spans.push(provider_span(
            spend_window_label(&window.window),
            ProviderTone::Muted,
            color,
        ));
        spans.push(provider_span(" ", ProviderTone::Default, color));
        spans.push(provider_span(
            format!(
                "{}/{}",
                format_provider_money(window.usage_usd),
                format_provider_money(window.limit_usd)
            ),
            value_tone,
            color,
        ));
    }
    provider_line(spans, width, color, selected)
}

fn provider_account_usage_line(
    usage: &aio_observer_protocol::ObserverProviderAccountUsage,
    width: usize,
    color: bool,
    selected: bool,
) -> Line<'static> {
    let (value, tone) = match (usage.state.as_str(), usage.amount) {
        ("available", Some(amount)) => (
            format_provider_account_amount(amount, usage.unit.as_deref()),
            ProviderTone::Success,
        ),
        ("available", None) => ("—".to_string(), ProviderTone::Muted),
        ("loading", _) => ("获取中".to_string(), ProviderTone::Warning),
        _ => ("获取失败".to_string(), ProviderTone::Error),
    };
    provider_line(
        vec![
            provider_span("余", ProviderTone::Label, color),
            provider_span(" ", ProviderTone::Default, color),
            provider_span(value, tone, color),
        ],
        width,
        color,
        selected,
    )
}

fn provider_availability_line(
    availability: &aio_observer_protocol::ObserverProviderAvailabilityTimeline,
    width: usize,
    color: bool,
    selected: bool,
) -> Line<'static> {
    let mut spans = vec![
        provider_span(
            format!("{}h", availability.hours),
            ProviderTone::Label,
            color,
        ),
        provider_span(" ", ProviderTone::Default, color),
    ];
    for bucket in availability.buckets.iter().take(12) {
        let tone = match bucket.state {
            aio_observer_protocol::ObserverProviderAvailabilityState::Healthy => {
                ProviderTone::Success
            }
            aio_observer_protocol::ObserverProviderAvailabilityState::Unhealthy => {
                ProviderTone::Error
            }
            aio_observer_protocol::ObserverProviderAvailabilityState::NoData => ProviderTone::Muted,
        };
        spans.push(provider_span("■", tone, color));
    }
    spans.extend([
        provider_span(" 成", ProviderTone::Default, color),
        provider_span(
            availability.success_count.to_string(),
            ProviderTone::Success,
            color,
        ),
        provider_span(" 败", ProviderTone::Default, color),
        provider_span(
            availability.failure_count.to_string(),
            ProviderTone::Error,
            color,
        ),
    ]);
    provider_line(spans, width, color, selected)
}

fn format_provider_money(value: f64) -> String {
    format!("${:.1}", value.max(0.0))
}

fn format_provider_account_amount(amount: f64, unit: Option<&str>) -> String {
    let amount = amount.max(0.0);
    match unit.map(str::trim).filter(|unit| !unit.is_empty()) {
        Some(unit) if matches!(unit, "$" | "¥" | "€" | "£") => {
            format!("{unit}{amount:.1}")
        }
        Some(unit) => format!("{amount:.1} {unit}"),
        None => format!("{amount:.1}"),
    }
}

fn provider_line(
    spans: Vec<Span<'static>>,
    width: usize,
    color: bool,
    selected: bool,
) -> Line<'static> {
    let mut line = truncate_provider_spans(spans, width, color);
    if selected {
        for span in &mut line.spans {
            span.style = selected_style(span.style, color);
        }
    }
    line
}

fn truncate_provider_spans(
    spans: Vec<Span<'static>>,
    max_width: usize,
    color: bool,
) -> Line<'static> {
    let width = spans
        .iter()
        .map(|span| UnicodeWidthStr::width(span.content.as_ref()))
        .sum::<usize>();
    if width <= max_width {
        return Line::from(spans);
    }
    if max_width == 0 {
        return Line::default();
    }
    let target = max_width.saturating_sub(1);
    let mut output = Vec::new();
    let mut used = 0_usize;
    'spans: for span in spans {
        let mut text = String::new();
        for grapheme in span.content.graphemes(true) {
            let next = UnicodeWidthStr::width(grapheme);
            if used.saturating_add(next) > target {
                if !text.is_empty() {
                    output.push(Span::styled(text, span.style));
                }
                break 'spans;
            }
            text.push_str(grapheme);
            used = used.saturating_add(next);
        }
        if !text.is_empty() {
            output.push(Span::styled(text, span.style));
        }
    }
    output.push(Span::styled(
        "…",
        provider_style(ProviderTone::Muted, color),
    ));
    Line::from(output)
}

fn provider_span(text: impl Into<String>, tone: ProviderTone, color: bool) -> Span<'static> {
    Span::styled(text.into(), provider_style(tone, color))
}

fn provider_separator(color: bool) -> Span<'static> {
    provider_span(" | ", ProviderTone::Muted, color)
}

fn provider_style(tone: ProviderTone, color: bool) -> Style {
    let tone = match tone {
        ProviderTone::Default => Tone::Default,
        ProviderTone::Label => Tone::Accent,
        ProviderTone::Accent => Tone::Provider,
        ProviderTone::Success => Tone::Success,
        ProviderTone::Warning => Tone::Warning,
        ProviderTone::Error => Tone::Error,
        ProviderTone::Muted => Tone::Muted,
    };
    Palette::detected(color).style(tone)
}

fn selected_style(style: Style, color: bool) -> Style {
    Palette::detected(color).selected(style)
}

fn provider_eligibility_tone(eligibility: &str) -> ProviderTone {
    match eligibility {
        "ready" => ProviderTone::Success,
        "half_open" | "spend_limited" | "oauth_limited" | "cooldown" => ProviderTone::Warning,
        "circuit_open" => ProviderTone::Error,
        _ => ProviderTone::Muted,
    }
}

fn circuit_tone(state: &str) -> ProviderTone {
    match state.trim().to_ascii_uppercase().as_str() {
        "CLOSED" => ProviderTone::Success,
        "HALF_OPEN" | "HALF-OPEN" => ProviderTone::Warning,
        "OPEN" => ProviderTone::Error,
        _ => ProviderTone::Muted,
    }
}

fn displayable_oauth_quota(
    provider: &ObserverProviderStatus,
) -> Option<&aio_observer_protocol::ObserverProviderOAuthQuota> {
    if provider.auth_kind != "oauth" {
        return None;
    }
    provider.oauth_quota.as_ref().filter(|quota| {
        non_empty(quota.short_label.as_deref()).is_some()
            || non_empty(quota.five_hour_text.as_deref()).is_some()
            || non_empty(quota.weekly_text.as_deref()).is_some()
    })
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn spend_window_label(window: &str) -> &str {
    match window {
        "daily" => "日",
        "weekly" => "周",
        "monthly" => "月",
        "total" => "总",
        _ => "5h",
    }
}

fn provider_eligibility_label(eligibility: &str) -> &'static str {
    match eligibility {
        "ready" => "可用",
        "half_open" => "半开探测",
        "provider_disabled" => "供应商停用",
        "not_in_route" => "不在路由",
        "route_disabled" => "路由停用",
        "spend_limited" => "消费限额",
        "oauth_limited" => "OAuth 限额",
        "circuit_open" => "熔断",
        "cooldown" => "冷却",
        "gateway_stopped" => "网关关闭",
        _ => "状态未知",
    }
}

fn item_separator_style(color: bool) -> Style {
    Palette::detected(color).style(Tone::Muted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aio_observer_protocol::{
        ObserverConfiguredModelRoute, ObserverGatewayStatus, ObserverPreferredProvider,
        ObserverProviderAccountUsage, ObserverProviderAvailabilityBucket,
        ObserverProviderAvailabilityState, ObserverProviderAvailabilityTimeline,
        ObserverProviderCollection, ObserverProviderOAuthQuota, ObserverProviderSpendWindow,
        ObserverRequestState, ObserverSection, ObserverTodayUsage, OBSERVER_PROTOCOL_VERSION,
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

    fn line_text(line: &Line<'_>) -> String {
        line.spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect()
    }

    fn span_color(line: &Line<'_>, text: &str) -> Option<Color> {
        line.spans
            .iter()
            .find(|span| span.content.as_ref() == text)
            .and_then(|span| span.style.fg)
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
            providers: None,
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
            visible_ttfb_ms: None,
            upstream_stream_duration_ms: None,
            upstream_stream_timing_version: 0,
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
            requested_reasoning_effort: None,
            configured_model_route: None,
        }
    }

    fn provider_status(id: i64, name: &str, preferred: bool) -> ObserverProviderStatus {
        ObserverProviderStatus {
            provider_id: id,
            cli_key: "codex".to_string(),
            provider_name: name.to_string(),
            route_rank: Some(id),
            provider_enabled: true,
            route_enabled: true,
            auth_kind: "oauth".to_string(),
            preferred,
            eligibility: if preferred { "ready" } else { "oauth_limited" }.to_string(),
            circuit_state: Some("CLOSED".to_string()),
            circuit_failure_count: Some(0),
            circuit_failure_threshold: Some(5),
            recover_at_unix: None,
            spend_windows: vec![ObserverProviderSpendWindow {
                window: "daily".to_string(),
                usage_usd: 1.0,
                limit_usd: 5.0,
            }],
            oauth_quota: Some(ObserverProviderOAuthQuota {
                short_label: Some("Plus".to_string()),
                five_hour_text: Some("80%".to_string()),
                weekly_text: Some("50%".to_string()),
                five_hour_reset_at_unix: None,
                weekly_reset_at_unix: None,
                checked_at_unix: 1,
            }),
            account_usage: None,
            availability: None,
        }
    }

    fn availability_timeline() -> ObserverProviderAvailabilityTimeline {
        ObserverProviderAvailabilityTimeline {
            hours: 6,
            bucket_minutes: 30,
            success_count: 24,
            failure_count: 3,
            buckets: (0..12)
                .map(|index| ObserverProviderAvailabilityBucket {
                    start_at_ms: index * 30 * 60_000,
                    end_at_ms: (index + 1) * 30 * 60_000,
                    success_count: u32::from(index == 0),
                    failure_count: u32::from(index == 1),
                    state: match index {
                        0 => ObserverProviderAvailabilityState::Healthy,
                        1 => ObserverProviderAvailabilityState::Unhealthy,
                        _ => ObserverProviderAvailabilityState::NoData,
                    },
                })
                .collect(),
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
    fn dashboard_header_is_shared_and_keeps_only_the_compact_summary() {
        let mut live = LiveState::new(CliScope::Codex);
        let mut snapshot = empty_snapshot(CliScope::Codex);
        snapshot.active_inference_count = 1;
        snapshot.preferred_provider = ObserverSection::ready(ObserverPreferredProvider {
            cli_key: "codex".to_string(),
            provider_name: "INPUT 大春".to_string(),
            circuit_state: "closed".to_string(),
        });
        snapshot.today = ObserverSection::ready(ObserverTodayUsage {
            total_tokens: 194_000_000,
            cost_usd: Some(118.69),
        });
        live.apply_snapshot(snapshot);

        assert_eq!(
            dashboard_header_lines(&live, 80),
            [
                "并发 1 | 首选 INPUT 大春".to_string(),
                "Codex | 今日 $118.69 | 194M".to_string(),
            ]
        );

        let backend = TestBackend::new(80, 2);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| {
                let area = frame.area();
                draw_header(frame, area, &live, true);
            })
            .expect("shared header");
        let header = rendered_non_space_symbols(&terminal);
        assert!(header.contains("并发1|首选INPUT大春Codex|今日$118.69|194M"));
        assert!(!header.contains("AIO"));
        assert!(!header.contains("请求"));
        assert!(!header.contains("供应商"));
    }

    #[test]
    fn dashboard_header_uses_bounded_placeholders_without_a_snapshot() {
        let live = LiveState::new(CliScope::Codex);

        assert_eq!(
            dashboard_header_lines(&live, 80),
            [
                "并发 — | 首选 —".to_string(),
                "Codex | 今日 — | —".to_string(),
            ]
        );
    }

    #[test]
    fn provider_availability_detail_uses_local_time_without_a_timezone_suffix() {
        assert_eq!(
            format_minute_with_offset(14 * 60 * 60_000 + 30 * 60_000, 8 * 60 * 60),
            "22:30"
        );
        assert_eq!(
            format_minute_with_offset(14 * 60 * 60_000 + 30 * 60_000, -5 * 60 * 60),
            "09:30"
        );

        let lines = provider_availability_detail_lines(&availability_timeline());
        assert!(lines.iter().any(|line| line.contains("成1 败0")));
        assert!(lines.iter().all(|line| !line.contains("UTC")));
    }

    #[test]
    fn busy_refresh_keeps_the_last_snapshot_and_labels_it_stale() {
        let mut state = LiveState::new(CliScope::Codex);
        state.apply_snapshot(empty_snapshot(CliScope::Codex));
        state.set_offline(OfflineReason::Busy);

        assert!(state.snapshot.is_some());
        assert!(state
            .stale_label()
            .is_some_and(|label| label.starts_with("观测繁忙 ")));
    }

    #[test]
    fn request_cards_use_semantic_lines_and_unselected_separators() {
        let backend = TestBackend::new(32, 18);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let mut state = LogsState::new(CliScope::Codex);
        let mut snapshot = empty_snapshot(CliScope::Codex);
        let mut routed = terminal_request("newest");
        routed.configured_model_route = Some(ObserverConfiguredModelRoute {
            source_model: "gpt-5".to_string(),
            effective_model: "gpt-5.6-terra".to_string(),
            reasoning_effort: None,
            policy_source: "global".to_string(),
            model_applied: true,
            reasoning_effort_applied: false,
        });
        let ordinary = terminal_request("older");
        assert_eq!(request_card_lines(&routed, 10, 32).len(), 6);
        assert_eq!(request_card_lines(&ordinary, 10, 32).len(), 5);
        snapshot.recent_requests = ObserverSection::ready(vec![routed, ordinary]);
        state.apply_snapshot(snapshot);
        terminal
            .draw(|frame| draw_logs(frame, &mut state))
            .expect("draw");

        let cells = terminal.backend().buffer().content();
        assert_eq!(cells[32 * 2].symbol(), "─");
        assert_eq!(cells[32 * 2].fg, Color::DarkGray);
        assert!(cells
            .iter()
            .any(|cell| cell.symbol() == "─" && cell.bg == Color::Reset));
        assert!(cells.iter().any(|cell| cell.fg == Color::Green));
        assert!(cells.iter().any(|cell| cell.fg == Color::Blue));
        assert!(cells.iter().any(|cell| cell.fg == Color::Magenta));
    }

    #[test]
    fn request_card_line_kinds_control_color_and_selection() {
        let request = terminal_request("semantic");
        assert_eq!(
            request_line_style(&request, RequestCardLineKind::Status, true, false).fg,
            Some(Color::Green)
        );
        assert_eq!(
            request_line_style(&request, RequestCardLineKind::Model, true, false).fg,
            Some(Color::Blue)
        );
        assert_eq!(
            request_line_style(&request, RequestCardLineKind::ModelTarget, true, false).fg,
            Some(Color::Blue)
        );
        assert_eq!(
            request_line_style(&request, RequestCardLineKind::Provider, true, false).fg,
            Some(Color::Magenta)
        );
        assert_eq!(
            request_line_style(&request, RequestCardLineKind::Route, true, false).fg,
            Some(Color::Green)
        );
        assert_eq!(
            request_line_style(&request, RequestCardLineKind::Metrics, true, false).fg,
            Some(Color::Cyan)
        );
        assert_eq!(
            request_line_style(&request, RequestCardLineKind::ModelTarget, true, true).bg,
            Some(Color::DarkGray)
        );
    }

    #[test]
    fn provider_view_uses_optional_oauth_line_and_semantic_colors() {
        let provider = provider_status(1, "首选供应商", true);
        let lines = provider_card_lines(&provider, 32, true, false);
        assert_eq!(lines.len(), 5);
        assert_eq!(span_color(&lines[1], "供"), Some(Color::Cyan));
        assert_eq!(span_color(&lines[1], "开"), Some(Color::Green));
        assert_eq!(span_color(&lines[2], "可用"), Some(Color::Green));
        assert_eq!(span_color(&lines[2], "熔"), Some(Color::Cyan));
        assert_eq!(span_color(&lines[2], "CLOSED"), Some(Color::Green));

        let mut without_oauth = provider.clone();
        without_oauth.oauth_quota = None;
        let lines = provider_card_lines(&without_oauth, 32, true, false);
        assert_eq!(lines.len(), 4);
        let has_oauth_quota_line = lines
            .iter()
            .any(|line| line_text(line).starts_with("OAuth "));
        assert!(!has_oauth_quota_line);

        let mut empty_oauth = provider.clone();
        empty_oauth.oauth_quota = Some(ObserverProviderOAuthQuota {
            short_label: Some("  ".to_string()),
            five_hour_text: None,
            weekly_text: None,
            five_hour_reset_at_unix: None,
            weekly_reset_at_unix: None,
            checked_at_unix: 1,
        });
        assert_eq!(provider_card_lines(&empty_oauth, 32, true, false).len(), 4);

        let mut api_key = provider.clone();
        api_key.auth_kind = "api_key".to_string();
        assert_eq!(provider_card_lines(&api_key, 32, true, false).len(), 4);

        let backend = TestBackend::new(32, 18);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let mut state = LogsState::new(CliScope::Codex);
        state.switch_view(DashboardView::Providers);
        let mut snapshot = empty_snapshot(CliScope::Codex);
        snapshot.providers = Some(ObserverSection::ready(ObserverProviderCollection {
            items: vec![provider, provider_status(2, "限额供应商", false)],
            truncated: false,
        }));
        state.apply_snapshot(snapshot);
        terminal
            .draw(|frame| draw_logs(frame, &mut state))
            .expect("draw");

        let text = rendered_non_space_symbols(&terminal);
        assert!(text.contains("供应商"));
        assert!(text.contains("首选"));
        let cells = terminal.backend().buffer().content();
        assert!(cells.iter().any(|cell| cell.fg == Color::Green));
        assert!(cells.iter().any(|cell| cell.fg == Color::Cyan));
    }

    #[test]
    fn provider_cards_hide_unconfigured_rows_and_keep_availability_last() {
        let mut provider = provider_status(1, "完整供应商", true);
        provider.spend_windows = vec![
            ObserverProviderSpendWindow {
                window: "five_hour".to_string(),
                usage_usd: 0.04,
                limit_usd: 100.0,
            },
            ObserverProviderSpendWindow {
                window: "daily".to_string(),
                usage_usd: 10.54,
                limit_usd: 200.04,
            },
        ];
        provider.account_usage = Some(ObserverProviderAccountUsage {
            state: "available".to_string(),
            amount: Some(12.54),
            unit: Some("$".to_string()),
            last_fetched_at_unix: Some(1),
        });
        provider.availability = Some(availability_timeline());

        let lines = provider_card_lines(&provider, 80, true, false);
        assert_eq!(lines.len(), 7);
        assert_eq!(line_text(&lines[3]), "限 5h $0.0/$100.0 日 $10.5/$200.0");
        assert_eq!(line_text(&lines[4]), "余 $12.5");
        assert!(line_text(lines.last().expect("availability line")).starts_with("6h ■■■"));
        assert!(line_text(lines.last().expect("availability line")).ends_with("成24 败3"));
        assert_eq!(
            lines.last().expect("availability line").spans[2].style.fg,
            Some(Color::Green)
        );
        assert_eq!(
            lines.last().expect("availability line").spans[3].style.fg,
            Some(Color::Red)
        );
        assert_eq!(
            lines.last().expect("availability line").spans[4].style.fg,
            Some(Color::DarkGray)
        );

        provider.spend_windows.clear();
        provider.account_usage = None;
        provider.oauth_quota = None;
        provider.availability = None;
        let minimal = provider_card_lines(&provider, 80, true, false);
        assert_eq!(minimal.len(), 3);
        assert!(minimal
            .iter()
            .all(|line| !line_text(line).starts_with('限')));
    }

    #[test]
    fn provider_account_row_reports_loading_and_failure_without_stale_amount() {
        let mut provider = provider_status(1, "余额供应商", true);
        provider.spend_windows.clear();
        provider.oauth_quota = None;
        provider.account_usage = Some(ObserverProviderAccountUsage {
            state: "loading".to_string(),
            amount: Some(99.0),
            unit: Some("credits".to_string()),
            last_fetched_at_unix: Some(1),
        });
        assert_eq!(
            line_text(&provider_card_lines(&provider, 40, true, false)[3]),
            "余 获取中"
        );

        provider.account_usage.as_mut().expect("usage").state = "failed".to_string();
        assert_eq!(
            line_text(&provider_card_lines(&provider, 40, true, false)[3]),
            "余 获取失败"
        );
    }

    #[test]
    fn provider_limit_and_circuit_states_use_distinct_colors() {
        let mut provider = provider_status(1, "状态供应商", false);
        provider.eligibility = "spend_limited".to_string();
        let limited = provider_card_lines(&provider, 40, true, false);
        assert_eq!(span_color(&limited[2], "消费限额"), Some(Color::Yellow));

        provider.eligibility = "circuit_open".to_string();
        provider.circuit_state = Some("OPEN".to_string());
        let open = provider_card_lines(&provider, 40, true, false);
        assert_eq!(span_color(&open[2], "熔断"), Some(Color::Red));
        assert_eq!(span_color(&open[2], "熔"), Some(Color::Red));
        assert_eq!(span_color(&open[2], "OPEN"), Some(Color::Red));

        provider.eligibility = "provider_disabled".to_string();
        provider.provider_enabled = false;
        let disabled = provider_card_lines(&provider, 40, true, false);
        assert_eq!(
            span_color(&disabled[2], "供应商停用"),
            Some(Color::DarkGray)
        );
        assert!(disabled[1]
            .spans
            .iter()
            .any(|span| span.content.as_ref() == "停" && span.style.fg == Some(Color::Red)));
    }

    #[test]
    fn dashboard_views_keep_independent_selection() {
        let mut state = LogsState::new(CliScope::Codex);
        let mut snapshot = empty_snapshot(CliScope::Codex);
        snapshot.recent_requests = ObserverSection::ready(vec![
            terminal_request("newest"),
            terminal_request("request-selected"),
        ]);
        snapshot.providers = Some(ObserverSection::ready(ObserverProviderCollection {
            items: vec![
                provider_status(1, "A", true),
                provider_status(2, "B", false),
            ],
            truncated: false,
        }));
        state.apply_snapshot(snapshot);
        let now = Instant::now();
        state.select_current(1, now);
        state.switch_view(DashboardView::Providers);
        state.select_current(1, now);
        state.switch_view(DashboardView::Requests);
        assert_eq!(state.selected, Some(1));
        state.switch_view(DashboardView::Providers);
        assert_eq!(state.provider_selected, Some(1));
        assert_eq!(
            state
                .selected_provider()
                .map(|provider| provider.provider_id),
            Some(2)
        );
    }

    #[test]
    fn inactive_selection_clears_after_five_seconds_and_refresh_does_not_extend_it() {
        let mut state = LogsState::new(CliScope::Codex);
        let mut snapshot = empty_snapshot(CliScope::Codex);
        snapshot.recent_requests = ObserverSection::ready(vec![
            terminal_request("newest"),
            terminal_request("selected"),
        ]);
        state.apply_snapshot(snapshot.clone());
        assert_eq!(state.selected, None);

        let selected_at = Instant::now();
        state.select_current(1, selected_at);
        state.apply_snapshot(snapshot);
        assert_eq!(
            state.selected_request().map(|request| request.key.as_str()),
            Some("selected")
        );
        assert!(!state.expire_inactive_selections(
            selected_at + SELECTION_IDLE_TIMEOUT - Duration::from_millis(1)
        ));
        assert!(state.expire_inactive_selections(selected_at + SELECTION_IDLE_TIMEOUT));
        assert_eq!(state.selected, None);
        assert_eq!(state.current_selection(), None);
    }

    #[test]
    fn navigation_after_selection_expiry_uses_the_newest_request_as_its_anchor() {
        let mut state = LogsState::new(CliScope::Codex);
        let mut snapshot = empty_snapshot(CliScope::Codex);
        snapshot.recent_requests = ObserverSection::ready(vec![
            terminal_request("newest"),
            terminal_request("older-1"),
            terminal_request("older-2"),
        ]);
        state.apply_snapshot(snapshot);
        let now = Instant::now();

        state.move_selection(-1, now);
        assert_eq!(state.selected, Some(0));

        state.clear_current_selection();
        state.move_selection(1, now);
        assert_eq!(state.selected, Some(1));

        state.clear_current_selection();
        state.move_selection(5, now);
        assert_eq!(state.selected, Some(2));
    }

    #[test]
    fn clearing_request_selection_restores_the_newest_visible_card() {
        let mut newest = terminal_request("newest");
        newest.provider_name = Some("NEWEST".to_string());
        let mut older = terminal_request("older");
        older.provider_name = Some("OLDER".to_string());
        let mut snapshot = empty_snapshot(CliScope::Codex);
        snapshot.recent_requests = ObserverSection::ready(vec![newest, older]);

        let backend = TestBackend::new(32, 10);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let mut state = LogsState::new(CliScope::Codex);
        state.apply_snapshot(snapshot);
        let selected_at = Instant::now();
        state.select_current(1, selected_at);
        terminal
            .draw(|frame| draw_logs(frame, &mut state))
            .expect("draw selected");
        let selected_text = rendered_non_space_symbols(&terminal);
        assert!(selected_text.contains("OLDER"));
        assert!(!selected_text.contains("NEWEST"));

        assert!(state.expire_inactive_selections(selected_at + SELECTION_IDLE_TIMEOUT));
        terminal
            .draw(|frame| draw_logs(frame, &mut state))
            .expect("draw newest");
        let newest_text = rendered_non_space_symbols(&terminal);
        assert!(newest_text.contains("NEWEST"));
        assert!(!newest_text.contains("OLDER"));
    }

    #[test]
    fn detail_suspends_selection_expiry_until_returning_to_list() {
        let mut state = LogsState::new(CliScope::Codex);
        let mut snapshot = empty_snapshot(CliScope::Codex);
        snapshot.recent_requests = ObserverSection::ready(vec![terminal_request("selected")]);
        state.apply_snapshot(snapshot);
        let selected_at = Instant::now();
        state.select_current(0, selected_at);
        state.suspend_current_selection_expiry();

        assert!(!state.expire_inactive_selections(
            selected_at + SELECTION_IDLE_TIMEOUT + Duration::from_secs(30)
        ));
        assert_eq!(state.selected, Some(0));

        let returned_at = selected_at + Duration::from_secs(60);
        state.resume_current_selection_expiry(returned_at);
        assert!(state.expire_inactive_selections(returned_at + SELECTION_IDLE_TIMEOUT));
        assert_eq!(state.selected, None);
    }

    #[test]
    fn provider_selection_expires_independently() {
        let mut state = LogsState::new(CliScope::Codex);
        state.switch_view(DashboardView::Providers);
        let mut snapshot = empty_snapshot(CliScope::Codex);
        snapshot.providers = Some(ObserverSection::ready(ObserverProviderCollection {
            items: vec![
                provider_status(1, "A", true),
                provider_status(2, "B", false),
            ],
            truncated: false,
        }));
        state.apply_snapshot(snapshot);
        let selected_at = Instant::now();
        state.select_current(1, selected_at);
        assert_eq!(state.provider_selected, Some(1));
        assert_eq!(state.provider_key, Some(2));

        assert!(state.expire_inactive_selections(selected_at + SELECTION_IDLE_TIMEOUT));
        assert_eq!(state.provider_selected, None);
        assert_eq!(state.provider_key, None);
    }

    #[test]
    fn provider_probe_state_deduplicates_and_isolates_results_by_provider() {
        let mut state = LogsState::new(CliScope::Codex);
        state.switch_view(DashboardView::Providers);
        let mut snapshot = empty_snapshot(CliScope::Codex);
        snapshot.providers = Some(ObserverSection::ready(ObserverProviderCollection {
            items: vec![
                provider_status(1, "A", true),
                provider_status(2, "B", false),
            ],
            truncated: false,
        }));
        state.apply_snapshot(snapshot);
        state.select_current(0, Instant::now());
        state.detail = true;

        assert_eq!(state.begin_provider_probe(), Some(1));
        assert_eq!(state.begin_provider_probe(), None);
        state.finish_provider_probe(2, Err(OfflineReason::Unreachable));
        assert!(matches!(
            state.provider_probes.get(&1),
            Some(ProviderProbeState::Running)
        ));
        state.finish_provider_probe(
            1,
            Ok(ObserverProviderAvailabilityTestResult {
                ok: true,
                provider_id: 1,
                provider_name: "A".to_string(),
                base_url: "https://example.com/v1".to_string(),
                status: Some(200),
                latency_ms: 123,
                error: None,
                response_preview: Some("ok".to_string()),
            }),
        );
        let result_lines =
            provider_probe_detail_lines(state.provider_probes.get(&1).expect("probe result"));
        assert!(result_lines.iter().any(|line| line == "结果：可用"));
        assert!(result_lines.iter().any(|line| line == "HTTP：200"));
        assert!(result_lines
            .iter()
            .any(|line| line == "Base URL：https://example.com/v1"));
        assert!(!state.provider_probes.contains_key(&2));

        state.select_current(1, Instant::now());
        assert_eq!(state.begin_provider_probe(), Some(2));
        assert!(matches!(
            state.provider_probes.get(&1),
            Some(ProviderProbeState::Complete(result)) if result.provider_id == 1
        ));
    }

    #[test]
    fn provider_view_reports_legacy_observer_without_dropping_logs() {
        let mut state = LogsState::new(CliScope::Codex);
        let mut snapshot = empty_snapshot(CliScope::Codex);
        snapshot.recent_requests = ObserverSection::ready(vec![terminal_request("kept")]);
        state.switch_view(DashboardView::Providers);
        state.apply_snapshot(snapshot);
        assert_eq!(
            provider_empty_message(&state),
            "当前 AIO 版本不支持供应商视图"
        );
        state.switch_view(DashboardView::Requests);
        assert_eq!(state.request_count(), 1);
    }

    #[test]
    fn one_row_status_keeps_a_visible_prefix() {
        let backend = TestBackend::new(24, 1);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let mut state = LiveState::new(CliScope::Codex);
        state.apply_snapshot(empty_snapshot(CliScope::Codex));
        terminal
            .draw(|frame| draw_status(frame, &state, &StatusItem::DEFAULT, true))
            .expect("draw");
        let text = rendered_non_space_symbols(&terminal);
        assert!(text.contains("首选"));
        let cells = terminal.backend().buffer().content();
        assert_eq!(cells[0].symbol(), " ");
        assert_eq!(cells[1].symbol(), " ");
        assert_ne!(cells[2].symbol(), " ");
    }

    #[test]
    fn statusline_uses_soft_codex_palette_and_dim_separator() {
        assert_eq!(
            status_tone_style(StatusTone::Scope, true).fg,
            Some(Color::Cyan)
        );
        assert_eq!(
            status_tone_style(StatusTone::Provider, true).fg,
            Some(Color::Magenta)
        );
        assert_eq!(
            status_tone_style(StatusTone::Tokens, true).fg,
            Some(Color::Green)
        );
        assert_eq!(
            status_tone_style(StatusTone::Error, true).fg,
            Some(Color::Red)
        );
        let separator = status_tone_style(StatusTone::Separator, true);
        assert_eq!(separator.fg, Some(Color::DarkGray));
        assert!(separator.add_modifier.contains(Modifier::DIM));
        assert!(!status_tone_style(StatusTone::Scope, true)
            .add_modifier
            .contains(Modifier::BOLD));
    }

    #[test]
    fn detail_labels_and_sections_use_the_shared_palette() {
        let lines = styled_detail_lines(
            vec![
                "状态  200 成功".to_string(),
                String::new(),
                "路由链".to_string(),
                "1. Provider".to_string(),
            ],
            true,
        );
        assert_eq!(lines[0].spans[0].style.fg, Some(Color::Cyan));
        assert_eq!(lines[2].style.fg, Some(Color::Cyan));
        assert!(lines[2].style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(lines[3].spans[0].style.fg, None);
    }

    #[test]
    fn wrapped_statusline_keeps_two_column_gutter_on_every_visible_row() {
        let width = 18_usize;
        let height = 4_usize;
        let backend = TestBackend::new(width as u16, height as u16);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let mut state = LiveState::new(CliScope::Codex);
        state.apply_snapshot(empty_snapshot(CliScope::Codex));
        terminal
            .draw(|frame| draw_status(frame, &state, &StatusItem::DEFAULT, true))
            .expect("draw");

        let cells = terminal.backend().buffer().content();
        let mut visible_rows = 0;
        for row in 0..height {
            let start = row * width;
            let visible = cells[start + 2..start + width]
                .iter()
                .any(|cell| !cell.symbol().trim().is_empty());
            if visible {
                visible_rows += 1;
                assert_eq!(cells[start].symbol(), " ");
                assert_eq!(cells[start + 1].symbol(), " ");
            }
        }
        assert!(visible_rows >= 2);
    }

    #[test]
    fn provider_styled_truncation_respects_cjk_width() {
        let provider = provider_status(1, "超长中文供应商名称", true);
        let lines = provider_card_lines(&provider, 10, true, false);
        assert!(lines.iter().all(|line| {
            line.spans
                .iter()
                .map(|span| UnicodeWidthStr::width(span.content.as_ref()))
                .sum::<usize>()
                <= 10
        }));
        assert!(lines.iter().any(|line| line_text(line).contains('…')));
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
        assert_eq!(singleton.notice.as_deref(), Some("至少保留一个状态栏项目"));
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
        state.select_current(1, Instant::now());

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
