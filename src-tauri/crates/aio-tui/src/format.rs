use crate::config::StatusItem;
use aio_observer_protocol::{CliScope, ObserverRequest, ObserverRequestState, ObserverSnapshotV1};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

pub fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or(0)
}

pub fn cli_label(cli_key: &str) -> &str {
    match cli_key.trim().to_ascii_lowercase().as_str() {
        "claude" => "Claude",
        "codex" => "Codex",
        "grok" => "Grok",
        "gemini" => "Gemini",
        "all" => "全部",
        _ => "未知",
    }
}

pub fn scope_label(scope: CliScope) -> &'static str {
    cli_label(scope.as_str())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusTone {
    Default,
    Success,
    Warning,
    Error,
    Scope,
    Provider,
    Model,
    Folder,
    Timing,
    Cost,
    Activity,
    Tokens,
    Version,
    Separator,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusSegment {
    pub text: String,
    pub tone: StatusTone,
}

impl StatusSegment {
    pub fn new(text: impl Into<String>, tone: StatusTone) -> Self {
        Self {
            text: text.into(),
            tone,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestCardLineKind {
    Status,
    Model,
    ModelTarget,
    Provider,
    Route,
    Metrics,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestCardLine {
    pub text: String,
    pub kind: RequestCardLineKind,
}

impl RequestCardLine {
    fn new(text: impl Into<String>, kind: RequestCardLineKind) -> Self {
        Self {
            text: text.into(),
            kind,
        }
    }
}

pub fn status_segments(snapshot: &ObserverSnapshotV1, items: &[StatusItem]) -> Vec<StatusSegment> {
    items
        .iter()
        .map(|item| status_segment(snapshot, *item))
        .collect()
}

fn status_segment(snapshot: &ObserverSnapshotV1, item: StatusItem) -> StatusSegment {
    match item {
        StatusItem::Gateway => {
            if snapshot.gateway.running {
                let text = snapshot
                    .gateway
                    .port
                    .map(|port| format!("网关 {port}"))
                    .unwrap_or_else(|| "网关 开启".to_string());
                StatusSegment::new(text, StatusTone::Success)
            } else {
                StatusSegment::new("网关 关闭", StatusTone::Error)
            }
        }
        StatusItem::Scope => StatusSegment::new(
            format!("范围 {}", scope_label(snapshot.scope)),
            StatusTone::Scope,
        ),
        StatusItem::PreferredProvider => {
            if !snapshot.preferred_provider.available {
                StatusSegment::new("首选 不可用", StatusTone::Warning)
            } else if let Some(provider) = snapshot.preferred_provider.value.as_ref() {
                let text = if snapshot.scope == CliScope::All {
                    format!(
                        "首选 {}/{}",
                        cli_label(&provider.cli_key),
                        provider.provider_name
                    )
                } else {
                    format!("首选 {}", provider.provider_name)
                };
                StatusSegment::new(text, StatusTone::Provider)
            } else {
                StatusSegment::new("首选 —", StatusTone::Default)
            }
        }
        StatusItem::LastRequest => last_request_field(snapshot, "上次", |request| {
            (
                format!(
                    "{} {} {}",
                    terminal_status(request),
                    request.provider_name.as_deref().unwrap_or("—"),
                    route_result(request)
                ),
                request_tone(request),
            )
        }),
        StatusItem::LastStatus => last_request_field(snapshot, "状态", |request| {
            (terminal_status(request), request_tone(request))
        }),
        StatusItem::LastProvider => last_request_field(snapshot, "上游", |request| {
            (
                request.provider_name.as_deref().unwrap_or("—").to_string(),
                StatusTone::Provider,
            )
        }),
        StatusItem::LastRoute => last_request_field(snapshot, "路由", |request| {
            (route_result(request), route_tone(request))
        }),
        StatusItem::LastModel => last_request_field(snapshot, "模型", |request| {
            (request_model(request), StatusTone::Model)
        }),
        StatusItem::LastFolder => last_request_field(snapshot, "目录", |request| {
            (
                request.folder_name.as_deref().unwrap_or("—").to_string(),
                StatusTone::Folder,
            )
        }),
        StatusItem::LastDuration => last_request_field(snapshot, "耗时", |request| {
            (
                request
                    .duration_ms
                    .map(format_duration)
                    .unwrap_or_else(|| "—".to_string()),
                StatusTone::Timing,
            )
        }),
        StatusItem::LastTtfb => last_request_field(snapshot, "首字", |request| {
            (
                request
                    .ttfb_ms
                    .map(format_duration)
                    .unwrap_or_else(|| "—".to_string()),
                StatusTone::Timing,
            )
        }),
        StatusItem::LastCost => last_request_field(snapshot, "费用", |request| {
            (
                request
                    .cost_usd
                    .map(format_cost)
                    .unwrap_or_else(|| "—".to_string()),
                StatusTone::Cost,
            )
        }),
        StatusItem::RecentProvider => {
            if !snapshot.dominant_provider.available {
                StatusSegment::new("最近 不可用", StatusTone::Warning)
            } else if let Some(provider) = snapshot.dominant_provider.value.as_ref() {
                StatusSegment::new(
                    format!("最近 {} *{}", provider.provider_name, provider.count),
                    StatusTone::Provider,
                )
            } else {
                StatusSegment::new("最近 —", StatusTone::Default)
            }
        }
        StatusItem::Concurrency => StatusSegment::new(
            format!("并发 {}", snapshot.active_inference_count),
            StatusTone::Activity,
        ),
        StatusItem::TodayCost => {
            if !snapshot.today.available {
                StatusSegment::new("今日 不可用", StatusTone::Warning)
            } else {
                StatusSegment::new(
                    format!(
                        "今日 {}",
                        snapshot
                            .today
                            .value
                            .as_ref()
                            .and_then(|value| value.cost_usd)
                            .map(format_cost)
                            .unwrap_or_else(|| "—".to_string())
                    ),
                    StatusTone::Cost,
                )
            }
        }
        StatusItem::TodayTokens => {
            if !snapshot.today.available {
                StatusSegment::new("Token 不可用", StatusTone::Warning)
            } else {
                StatusSegment::new(
                    format!(
                        "Token {}",
                        snapshot
                            .today
                            .value
                            .as_ref()
                            .map(|value| format_tokens(value.total_tokens))
                            .unwrap_or_else(|| "—".to_string())
                    ),
                    StatusTone::Tokens,
                )
            }
        }
        StatusItem::AppVersion => {
            StatusSegment::new(format!("AIO {}", snapshot.app_version), StatusTone::Version)
        }
    }
}

fn last_request_field(
    snapshot: &ObserverSnapshotV1,
    label: &str,
    render: impl FnOnce(&ObserverRequest) -> (String, StatusTone),
) -> StatusSegment {
    if !snapshot.last_request.available {
        return StatusSegment::new(format!("{label} 不可用"), StatusTone::Warning);
    }
    let Some(request) = snapshot.last_request.value.as_ref() else {
        return StatusSegment::new(format!("{label} —"), StatusTone::Default);
    };
    let (value, tone) = render(request);
    StatusSegment::new(format!("{label} {value}"), tone)
}

fn request_tone(request: &ObserverRequest) -> StatusTone {
    if request.state == ObserverRequestState::Active {
        StatusTone::Activity
    } else if request.interrupted {
        StatusTone::Warning
    } else if request
        .status
        .is_some_and(|status| (200..300).contains(&status))
    {
        StatusTone::Success
    } else {
        StatusTone::Error
    }
}

fn route_tone(request: &ObserverRequest) -> StatusTone {
    if request.provider_switch_count > 0 || request.retry_count > 0 {
        StatusTone::Warning
    } else if request.attempt_count > 0 {
        StatusTone::Success
    } else {
        StatusTone::Default
    }
}

pub fn status_plain(snapshot: &ObserverSnapshotV1, items: &[StatusItem]) -> String {
    status_segments(snapshot, items)
        .into_iter()
        .map(|segment| segment.text)
        .collect::<Vec<_>>()
        .join(" | ")
}

pub fn wrap_status_segments(segments: &[StatusSegment], width: usize) -> Vec<Vec<StatusSegment>> {
    if width == 0 {
        return Vec::new();
    }
    let mut lines = Vec::new();
    let mut current = Vec::new();
    let mut current_width = 0_usize;
    for segment in segments {
        let pieces = wrap_display(&segment.text, width);
        for (index, piece) in pieces.into_iter().enumerate() {
            if index > 0 && !current.is_empty() {
                lines.push(std::mem::take(&mut current));
                current_width = 0;
            }
            let separator_width = if current.is_empty() { 0 } else { 3 };
            let candidate_width = current_width
                .saturating_add(separator_width)
                .saturating_add(display_width(&piece));
            if candidate_width <= width {
                if !current.is_empty() {
                    current.push(StatusSegment::new(" · ", StatusTone::Separator));
                }
                current.push(StatusSegment::new(piece, segment.tone));
                current_width = candidate_width;
            } else {
                if !current.is_empty() {
                    lines.push(std::mem::take(&mut current));
                }
                current_width = display_width(&piece);
                current = vec![StatusSegment::new(piece, segment.tone)];
            }
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

pub fn truncate_status_line(line: &[StatusSegment], max_width: usize) -> Vec<StatusSegment> {
    let width = line
        .iter()
        .map(|segment| display_width(&segment.text))
        .sum::<usize>();
    if width <= max_width {
        return line.to_vec();
    }
    if max_width == 0 {
        return Vec::new();
    }
    let target = max_width - 1;
    let mut output = Vec::new();
    let mut used = 0_usize;
    'segments: for segment in line {
        for grapheme in segment.text.graphemes(true) {
            let next = grapheme.width();
            if used.saturating_add(next) > target {
                break 'segments;
            }
            push_status_text(&mut output, grapheme, segment.tone);
            used = used.saturating_add(next);
        }
    }
    push_status_text(&mut output, "…", StatusTone::Separator);
    output
}

fn push_status_text(output: &mut Vec<StatusSegment>, text: &str, tone: StatusTone) {
    if let Some(last) = output.last_mut().filter(|last| last.tone == tone) {
        last.text.push_str(text);
    } else {
        output.push(StatusSegment::new(text, tone));
    }
}

pub fn request_card_lines(
    request: &ObserverRequest,
    now_ms: i64,
    width: usize,
) -> Vec<RequestCardLine> {
    let compaction = request
        .context_compaction
        .as_ref()
        .map(|marker| match marker.mode.as_str() {
            "local" => " 压缩·本地",
            "remote" => " 压缩·远程",
            _ => " 压缩·未知",
        })
        .unwrap_or_default();
    let folder = request.folder_name.as_deref().unwrap_or("无目录");
    let provider = request.provider_name.as_deref().unwrap_or("未上游");
    let duration = request
        .duration_ms
        .map(format_duration)
        .unwrap_or_else(|| "—".to_string());
    let (input, output, cache) = request
        .usage
        .as_ref()
        .map(|usage| {
            (
                usage
                    .input_tokens
                    .map(format_tokens)
                    .unwrap_or_else(|| "—".to_string()),
                usage
                    .output_tokens
                    .map(format_tokens)
                    .unwrap_or_else(|| "—".to_string()),
                usage
                    .cache_read_tokens
                    .unwrap_or(0)
                    .saturating_add(usage.cache_creation_tokens.unwrap_or(0)),
            )
        })
        .unwrap_or_else(|| ("—".to_string(), "—".to_string(), 0));
    let cost = request
        .cost_usd
        .map(format_cost)
        .unwrap_or_else(|| "—".to_string());
    let route_summary = match output_tokens_per_second(request) {
        Some(rate) => format!(
            "{}  {}  {}",
            route_result(request),
            duration,
            format_tokens_per_second_short(rate)
        ),
        None => format!("{}  {}", route_result(request), duration),
    };
    let mut lines = vec![RequestCardLine::new(
        truncate_display(
            &format!(
                "{}  {}",
                request_status_label(request),
                relative_time(request.created_at_ms, now_ms)
            ),
            width,
        ),
        RequestCardLineKind::Status,
    )];
    lines.extend(request_card_model_lines(request, compaction, width));
    lines.extend([
        RequestCardLine::new(
            truncate_display(&format!("{}  {}", provider, folder), width),
            RequestCardLineKind::Provider,
        ),
        RequestCardLine::new(
            truncate_display(&route_summary, width),
            RequestCardLineKind::Route,
        ),
        RequestCardLine::new(
            truncate_display(
                &format!(
                    "I {} O {} C {} {}",
                    input,
                    output,
                    format_tokens(cache),
                    cost
                ),
                width,
            ),
            RequestCardLineKind::Metrics,
        ),
    ]);
    lines
}

fn request_card_model_lines(
    request: &ObserverRequest,
    compaction: &str,
    width: usize,
) -> Vec<RequestCardLine> {
    let Some(route) = request
        .configured_model_route
        .as_ref()
        .filter(|route| configured_model_route_is_valid(route))
    else {
        let model = request_model_with_requested_effort(request);
        return vec![RequestCardLine::new(
            truncate_display(
                &format!("{} / {}{}", cli_label(&request.cli_key), model, compaction),
                width,
            ),
            RequestCardLineKind::Model,
        )];
    };
    let is_codex = request.cli_key == "codex";
    let requested_effort = if is_codex {
        request.requested_reasoning_effort.as_deref()
    } else {
        None
    };
    let effective_effort = if is_codex {
        if route.reasoning_effort_applied {
            route.reasoning_effort.as_deref()
        } else {
            requested_effort
        }
    } else {
        None
    };
    if !route.model_applied || route.source_model == route.effective_model {
        let model = if is_codex {
            model_with_effort(&route.source_model, effective_effort)
        } else if route.reasoning_effort_applied {
            format!(
                "{}·{}",
                route.source_model,
                route.reasoning_effort.as_deref().unwrap_or("未知")
            )
        } else {
            route.source_model.clone()
        };
        return vec![RequestCardLine::new(
            truncate_display(
                &format!("{} / {}{}", cli_label(&request.cli_key), model, compaction),
                width,
            ),
            RequestCardLineKind::Model,
        )];
    }

    let source_model = if is_codex {
        model_with_effort(&route.source_model, requested_effort)
    } else {
        route.source_model.clone()
    };
    let source = truncate_with_trailing_arrow(
        &format!("{} / {} ", cli_label(&request.cli_key), source_model),
        width,
    );
    let target_model = if is_codex {
        model_with_effort(&route.effective_model, effective_effort)
    } else if route.reasoning_effort_applied {
        format!(
            "{}·{}",
            route.effective_model,
            route.reasoning_effort.as_deref().unwrap_or("未知")
        )
    } else {
        route.effective_model.clone()
    };
    let target = right_align_display(&format!("{}{}", target_model, compaction), width);
    vec![
        RequestCardLine::new(source, RequestCardLineKind::Model),
        RequestCardLine::new(target, RequestCardLineKind::ModelTarget),
    ]
}

fn configured_model_route_is_valid(
    route: &aio_observer_protocol::ObserverConfiguredModelRoute,
) -> bool {
    display_width(route.source_model.trim()) > 0
        && display_width(route.effective_model.trim()) > 0
        && (!route.reasoning_effort_applied
            || route
                .reasoning_effort
                .as_deref()
                .is_some_and(|effort| display_width(effort.trim()) > 0))
        && matches!(route.policy_source.as_str(), "global" | "provider")
}

fn truncate_with_trailing_arrow(lead: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    if width == 1 {
        return "→".to_string();
    }
    let lead = truncate_display_prefix(lead, width - 1);
    format!("{lead}→")
}

fn right_align_display(value: &str, width: usize) -> String {
    let rendered = truncate_display(value, width);
    format!(
        "{}{}",
        " ".repeat(width.saturating_sub(display_width(&rendered))),
        rendered
    )
}

fn truncate_display_prefix(value: &str, max_width: usize) -> String {
    let mut output = String::new();
    let mut used = 0_usize;
    for grapheme in value.graphemes(true) {
        let next = grapheme.width();
        if used.saturating_add(next) > max_width {
            break;
        }
        output.push_str(grapheme);
        used = used.saturating_add(next);
    }
    output
}

pub fn detail_lines(request: &ObserverRequest, now_ms: i64) -> Vec<String> {
    let mut lines = vec![
        format!("状态  {}", request_status_label(request)),
        format!("时间  {}", relative_time(request.created_at_ms, now_ms)),
        format!("CLI   {}", cli_label(&request.cli_key)),
        format!("模型  {}", request_model(request)),
        format!("目录  {}", request.folder_name.as_deref().unwrap_or("—")),
        format!(
            "供应商  {}",
            request.provider_name.as_deref().unwrap_or("—")
        ),
        format!("路由  {}", route_result(request)),
        format!("方法  {} {}", request.method, request.path),
        format!(
            "耗时  {}",
            request
                .duration_ms
                .map(format_duration)
                .unwrap_or_else(|| "—".to_string())
        ),
        format!(
            "首字  {}",
            request
                .ttfb_ms
                .map(format_duration)
                .unwrap_or_else(|| "—".to_string())
        ),
        format!(
            "尝试  {}（重试 {}）",
            request.attempt_count, request.retry_count
        ),
        format!("错误码  {}", request.error_code.as_deref().unwrap_or("—")),
        format!("Session  {}", request.session_id.as_deref().unwrap_or("—")),
    ];
    if let Some(route) = request.configured_model_route.as_ref() {
        let source = match route.policy_source.as_str() {
            "provider" => "供应商覆盖",
            "global" => "全局",
            _ => "未知",
        };
        lines.insert(4, format!("路由规则  {source}"));
    }
    if let Some(usage) = request.usage.as_ref() {
        lines.extend([
            format!(
                "输入  {}",
                usage
                    .input_tokens
                    .map(format_tokens)
                    .unwrap_or_else(|| "—".to_string())
            ),
            format!(
                "输出  {}",
                usage
                    .output_tokens
                    .map(format_tokens)
                    .unwrap_or_else(|| "—".to_string())
            ),
            format!(
                "缓存读  {}",
                usage
                    .cache_read_tokens
                    .map(format_tokens)
                    .unwrap_or_else(|| "—".to_string())
            ),
            format!(
                "缓存写  {}",
                usage
                    .cache_creation_tokens
                    .map(format_tokens)
                    .unwrap_or_else(|| "—".to_string())
            ),
        ]);
    }
    lines.push(format!(
        "费用  {}",
        request
            .cost_usd
            .map(format_cost)
            .unwrap_or_else(|| "—".to_string())
    ));
    if let Some(marker) = request.context_compaction.as_ref() {
        lines.extend([
            String::new(),
            "上下文压缩".to_string(),
            format!("模式  {}", marker.mode),
            format!("实现  {}", marker.implementation),
            format!("触发  {}", marker.trigger),
            format!("原因  {}", marker.reason),
            format!("阶段  {}", marker.phase),
            format!("策略  {}", marker.strategy),
        ]);
    }
    if !request.route.is_empty() {
        lines.push(String::new());
        lines.push("路由链".to_string());
        for (index, hop) in request.route.iter().enumerate() {
            lines.push(format!(
                "{}. {} ×{} {} {}",
                index + 1,
                hop.provider_name,
                hop.attempts,
                hop.status
                    .map(|status| status.to_string())
                    .unwrap_or_else(|| "—".to_string()),
                hop.error_code.as_deref().unwrap_or("")
            ));
        }
    }
    lines
}

fn request_model(request: &ObserverRequest) -> String {
    let Some(route) = request
        .configured_model_route
        .as_ref()
        .filter(|route| configured_model_route_is_valid(route))
    else {
        return request_model_with_requested_effort(request);
    };
    if request.cli_key != "codex" {
        let model = if route.model_applied && route.source_model != route.effective_model {
            format!("{}→{}", route.source_model, route.effective_model)
        } else {
            route.source_model.clone()
        };
        return if route.reasoning_effort_applied {
            format!(
                "{}·思考{}",
                model,
                route.reasoning_effort.as_deref().unwrap_or("未知")
            )
        } else {
            model
        };
    }

    let requested_effort = request.requested_reasoning_effort.as_deref();
    let effective_effort = if route.reasoning_effort_applied {
        route.reasoning_effort.as_deref()
    } else {
        requested_effort
    };
    let source = model_with_effort(&route.source_model, requested_effort);
    if route.model_applied && route.source_model != route.effective_model {
        format!(
            "{}→{}",
            source,
            model_with_effort(&route.effective_model, effective_effort)
        )
    } else {
        model_with_effort(&route.source_model, effective_effort)
    }
}

fn request_model_with_requested_effort(request: &ObserverRequest) -> String {
    let Some(model) = request.model.as_deref() else {
        return "—".to_string();
    };
    if request.cli_key == "codex" {
        model_with_effort(model, request.requested_reasoning_effort.as_deref())
    } else {
        model.to_string()
    }
}

fn model_with_effort(model: &str, effort: Option<&str>) -> String {
    let Some(effort) = effort.map(str::trim).filter(|effort| !effort.is_empty()) else {
        return model.to_string();
    };
    let effort = effort.to_ascii_lowercase();
    if !matches!(
        effort.as_str(),
        "none" | "minimal" | "low" | "medium" | "high" | "xhigh" | "max" | "ultra"
    ) {
        return model.to_string();
    }
    let suffix = format!("-{effort}");
    if model.to_ascii_lowercase().ends_with(&suffix) {
        model.to_string()
    } else {
        format!("{model}-{effort}")
    }
}

pub fn request_status_label(request: &ObserverRequest) -> String {
    match request.state {
        ObserverRequestState::Active => "进行中".to_string(),
        ObserverRequestState::Terminal if request.interrupted => "499 已中断".to_string(),
        ObserverRequestState::Terminal => match request.status {
            Some(status) if (200..300).contains(&status) => format!("{status} 成功"),
            Some(status) => format!("{status} 失败"),
            None if request.error_code.is_some() => "请求失败".to_string(),
            None => "状态未知".to_string(),
        },
    }
}

pub fn route_result(request: &ObserverRequest) -> String {
    if request.provider_switch_count > 0 {
        if request.retry_count > 0 {
            return format!(
                "切换{}/重试{}",
                request.provider_switch_count, request.retry_count
            );
        }
        return format!("切换{}", request.provider_switch_count);
    }
    if request.retry_count > 0 {
        return format!("重试{}", request.retry_count);
    }
    if request.attempt_count > 0 {
        return "直连".to_string();
    }
    "未上游".to_string()
}

pub fn terminal_status(request: &ObserverRequest) -> String {
    if request.interrupted {
        return "499".to_string();
    }
    request
        .status
        .map(|status| status.to_string())
        .unwrap_or_else(|| "ERR".to_string())
}

pub fn format_duration(milliseconds: i64) -> String {
    let milliseconds = milliseconds.max(0);
    if milliseconds < 1000 {
        return format!("{milliseconds}ms");
    }
    if milliseconds < 60_000 {
        return format!("{:.1}s", milliseconds as f64 / 1000.0);
    }
    let minutes = milliseconds / 60_000;
    let seconds = (milliseconds % 60_000) / 1000;
    format!("{minutes}m{seconds:02}s")
}

pub fn format_tokens(value: i64) -> String {
    let value = value.max(0) as f64;
    if value >= 1_000_000_000.0 {
        format_compact(value / 1_000_000_000.0, "B")
    } else if value >= 1_000_000.0 {
        format_compact(value / 1_000_000.0, "M")
    } else if value >= 1_000.0 {
        format_compact(value / 1_000.0, "K")
    } else {
        format!("{}", value as i64)
    }
}

fn format_compact(value: f64, suffix: &str) -> String {
    if value >= 100.0 {
        format!("{value:.0}{suffix}")
    } else if value >= 10.0 {
        format!("{value:.1}{suffix}")
    } else {
        format!("{value:.2}{suffix}")
    }
}

pub fn format_cost(value: f64) -> String {
    if value >= 100.0 {
        format!("${value:.2}")
    } else if value >= 1.0 {
        format!("${value:.3}")
    } else {
        format!("${value:.6}")
    }
}

pub fn output_tokens_per_second(request: &ObserverRequest) -> Option<f64> {
    if request.state != ObserverRequestState::Terminal
        || request.final_upstream_attempt_timing_version != 1
        || request.error_code.is_some()
        || !request
            .status
            .is_some_and(|status| (200..300).contains(&status))
    {
        return None;
    }
    let output_tokens = request.usage.as_ref()?.output_tokens?;
    let final_upstream_attempt_duration_ms = request.final_upstream_attempt_duration_ms?;
    if output_tokens <= 0 || final_upstream_attempt_duration_ms <= 0 {
        return None;
    }
    let rate = output_tokens as f64 / (final_upstream_attempt_duration_ms as f64 / 1_000.0);
    rate.is_finite().then_some(rate)
}

pub fn format_tokens_per_second_short(value: f64) -> String {
    let value = value.max(0.0);
    if value >= 1_000.0 {
        format!("{:.1}k t/s", value / 1_000.0)
    } else {
        format!("{value:.1} t/s")
    }
}

pub fn relative_time(created_at_ms: i64, now_ms: i64) -> String {
    let elapsed = now_ms.saturating_sub(created_at_ms).max(0);
    if elapsed < 60_000 {
        return "<1分钟".to_string();
    }
    if elapsed < 3_600_000 {
        return format!("{}分钟", elapsed / 60_000);
    }
    if elapsed < 86_400_000 {
        return format!("{}小时", elapsed / 3_600_000);
    }
    format!("{}天", elapsed / 86_400_000)
}

pub fn truncate_display(value: &str, max_width: usize) -> String {
    if display_width(value) <= max_width {
        return value.to_string();
    }
    if max_width == 0 {
        return String::new();
    }
    if max_width == 1 {
        return "…".to_string();
    }
    let target = max_width - 1;
    let mut out = String::new();
    let mut width: usize = 0;
    for grapheme in value.graphemes(true) {
        let next = grapheme.width();
        if width.saturating_add(next) > target {
            break;
        }
        out.push_str(grapheme);
        width += next;
    }
    out.push('…');
    out
}

fn wrap_display(value: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return Vec::new();
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut width: usize = 0;
    for grapheme in value.graphemes(true) {
        let next = grapheme.width();
        if next > max_width {
            if !current.is_empty() {
                lines.push(std::mem::take(&mut current));
            }
            lines.push(truncate_display(grapheme, max_width));
            width = 0;
            continue;
        }
        if !current.is_empty() && width.saturating_add(next) > max_width {
            lines.push(std::mem::take(&mut current));
            width = 0;
        }
        current.push_str(grapheme);
        width = width.saturating_add(next);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

fn display_width(value: &str) -> usize {
    UnicodeWidthStr::width(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aio_observer_protocol::{
        ObserverConfiguredModelRoute, ObserverContextCompaction, ObserverDominantProvider,
        ObserverGatewayStatus, ObserverPreferredProvider, ObserverRequestUsage, ObserverSection,
        ObserverTodayUsage, OBSERVER_PROTOCOL_VERSION,
    };

    fn request_with_route_counts(
        attempt_count: u32,
        retry_count: u32,
        provider_switch_count: u32,
    ) -> ObserverRequest {
        ObserverRequest {
            key: "request-1".to_string(),
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
            final_upstream_attempt_duration_ms: None,
            final_upstream_attempt_timing_version: 0,
            attempt_count,
            retry_count,
            provider_switch_count,
            has_failover: provider_switch_count > 0,
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

    fn status_snapshot() -> ObserverSnapshotV1 {
        let mut request = request_with_route_counts(3, 1, 1);
        request.provider_name = Some("happy".to_string());
        request.model = Some("gpt-5.6-sol".to_string());
        request.folder_name = Some("aio-coding-hub".to_string());
        request.duration_ms = Some(12_345);
        request.ttfb_ms = Some(2_345);
        request.cost_usd = Some(0.123456);
        ObserverSnapshotV1 {
            protocol_version: OBSERVER_PROTOCOL_VERSION,
            app_version: "0.60.39".to_string(),
            generated_at_ms: 1,
            scope: CliScope::Codex,
            gateway: ObserverGatewayStatus {
                running: true,
                port: Some(37123),
            },
            preferred_provider: ObserverSection::ready(ObserverPreferredProvider {
                cli_key: "codex".to_string(),
                provider_name: "preferred".to_string(),
                circuit_state: "closed".to_string(),
            }),
            last_request: ObserverSection::ready(request),
            dominant_provider: ObserverSection::ready(ObserverDominantProvider {
                provider_name: "happy".to_string(),
                count: 7,
                sample_size: 10,
            }),
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

    #[test]
    fn truncation_respects_cjk_display_width() {
        assert_eq!(truncate_display("供应商OpenAI", 8), "供应商O…");
        assert!(display_width(&truncate_display("供应商OpenAI", 8)) <= 8);
    }

    #[test]
    fn selected_status_items_keep_requested_order_and_plain_output() {
        let snapshot = status_snapshot();
        let items = [
            StatusItem::LastModel,
            StatusItem::Gateway,
            StatusItem::Concurrency,
            StatusItem::TodayCost,
        ];
        let segments = status_segments(&snapshot, &items);
        assert_eq!(
            segments
                .iter()
                .map(|segment| segment.text.as_str())
                .collect::<Vec<_>>(),
            vec!["模型 gpt-5.6-sol", "网关 37123", "并发 13", "今日 $12.340"]
        );
        assert_eq!(
            status_plain(&snapshot, &items),
            "模型 gpt-5.6-sol | 网关 37123 | 并发 13 | 今日 $12.340"
        );
        assert_eq!(segments[0].tone, StatusTone::Model);
        assert_eq!(segments[1].tone, StatusTone::Success);
    }

    #[test]
    fn every_status_catalog_item_has_a_projection() {
        let segments = status_segments(&status_snapshot(), &StatusItem::ALL);
        assert_eq!(segments.len(), StatusItem::ALL.len());
        assert!(segments.iter().all(|segment| !segment.text.is_empty()));
    }

    #[test]
    fn status_segments_wrap_at_boundaries_first() {
        let lines = wrap_status_segments(
            &[
                StatusSegment::new("首选 Provider", StatusTone::Provider),
                StatusSegment::new("并发 13", StatusTone::Activity),
                StatusSegment::new("今日 $1.23", StatusTone::Cost),
            ],
            16,
        );
        let text = lines
            .iter()
            .map(|line| {
                line.iter()
                    .map(|segment| segment.text.as_str())
                    .collect::<String>()
            })
            .collect::<Vec<_>>();
        assert_eq!(text, vec!["首选 Provider", "并发 13", "今日 $1.23"]);
        assert_eq!(lines[0][0].tone, StatusTone::Provider);
        assert_eq!(lines[1][0].tone, StatusTone::Activity);
        assert_eq!(lines[2][0].tone, StatusTone::Cost);
    }

    #[test]
    fn styled_status_wrap_preserves_cjk_width_and_separator_tone() {
        let lines = wrap_status_segments(
            &[
                StatusSegment::new("模型 编程助手", StatusTone::Model),
                StatusSegment::new("今日 $1.23", StatusTone::Cost),
                StatusSegment::new("并发 13", StatusTone::Activity),
            ],
            26,
        );
        assert!(lines.iter().all(|line| {
            line.iter()
                .map(|segment| display_width(&segment.text))
                .sum::<usize>()
                <= 26
        }));
        assert!(lines
            .iter()
            .flatten()
            .any(|segment| { segment.text == " · " && segment.tone == StatusTone::Separator }));
        assert!(lines
            .iter()
            .flatten()
            .any(|segment| segment.tone == StatusTone::Model));
        assert!(lines
            .iter()
            .flatten()
            .any(|segment| segment.tone == StatusTone::Cost));
    }

    #[test]
    fn styled_truncation_keeps_tones_and_fits() {
        let line = vec![
            StatusSegment::new("模型 中文模型", StatusTone::Model),
            StatusSegment::new(" · ", StatusTone::Separator),
            StatusSegment::new("今日 $12.34", StatusTone::Cost),
        ];
        let truncated = truncate_status_line(&line, 12);
        let width = truncated
            .iter()
            .map(|segment| display_width(&segment.text))
            .sum::<usize>();
        assert!(width <= 12);
        assert_eq!(
            truncated.last().map(|segment| segment.text.as_str()),
            Some("…")
        );
        assert_eq!(truncated[0].tone, StatusTone::Model);
    }

    #[test]
    fn compact_units_stay_stable() {
        assert_eq!(format_tokens(999), "999");
        assert_eq!(format_tokens(1_500), "1.50K");
        assert_eq!(format_tokens(507_900_000), "508M");
    }

    #[test]
    fn route_result_keeps_switches_and_retries_independent() {
        assert_eq!(
            route_result(&request_with_route_counts(3, 1, 1)),
            "切换1/重试1"
        );
        assert_eq!(route_result(&request_with_route_counts(2, 0, 1)), "切换1");
        assert_eq!(route_result(&request_with_route_counts(3, 2, 0)), "重试2");
        assert_eq!(route_result(&request_with_route_counts(0, 0, 0)), "未上游");
    }

    #[test]
    fn request_card_keeps_provider_before_folder_and_appends_output_rate() {
        let mut request = request_with_route_counts(1, 0, 0);
        request.provider_name = Some("INPUT 大春".to_string());
        request.folder_name = Some("aio-coding-hub".to_string());
        request.duration_ms = Some(2_000);
        request.ttfb_ms = Some(500);
        request.final_upstream_attempt_duration_ms = Some(2_000);
        request.final_upstream_attempt_timing_version = 1;
        request.usage = Some(ObserverRequestUsage {
            input_tokens: Some(611),
            output_tokens: Some(200),
            total_tokens: Some(811),
            cache_read_tokens: Some(107_000),
            cache_creation_tokens: None,
        });

        let lines = request_card_lines(&request, 60_001, 80);
        assert_eq!(lines.len(), 5);
        assert_eq!(lines[1].kind, RequestCardLineKind::Model);
        assert_eq!(lines[1].text, "Codex / gpt-5");
        assert_eq!(lines[2].text, "INPUT 大春  aio-coding-hub");
        assert_eq!(lines[3].text, "直连  2.0s  100.0 t/s");
        assert!(request_card_lines(&request, 60_001, 15)[2]
            .text
            .starts_with("INPUT 大春"));
    }

    #[test]
    fn output_rate_uses_final_upstream_attempt_and_visibility_rules() {
        let mut request = request_with_route_counts(1, 0, 0);
        request.duration_ms = Some(20_000);
        request.ttfb_ms = Some(19_800);
        request.upstream_stream_duration_ms = Some(200);
        request.upstream_stream_timing_version = 1;
        request.final_upstream_attempt_duration_ms = Some(20_000);
        request.final_upstream_attempt_timing_version = 1;
        request.usage = Some(ObserverRequestUsage {
            input_tokens: None,
            output_tokens: Some(1_200),
            total_tokens: None,
            cache_read_tokens: None,
            cache_creation_tokens: None,
        });
        assert_eq!(output_tokens_per_second(&request), Some(60.0));
        assert_eq!(format_tokens_per_second_short(1_500.0), "1.5k t/s");

        request.duration_ms = Some(29_520);
        request.ttfb_ms = Some(29_360);
        request.upstream_stream_duration_ms = Some(160);
        request.final_upstream_attempt_duration_ms = Some(29_520);
        request.usage.as_mut().expect("usage").output_tokens = Some(439);
        assert!(output_tokens_per_second(&request).is_some_and(|rate| rate < 15.0));

        request.status = Some(500);
        assert_eq!(output_tokens_per_second(&request), None);
        request.status = Some(200);
        request.state = ObserverRequestState::Active;
        assert_eq!(output_tokens_per_second(&request), None);
        request.state = ObserverRequestState::Terminal;
        request.final_upstream_attempt_duration_ms = None;
        assert_eq!(output_tokens_per_second(&request), None);
        request.final_upstream_attempt_duration_ms = Some(29_520);
        request.final_upstream_attempt_timing_version = 0;
        assert_eq!(output_tokens_per_second(&request), None);
    }

    #[test]
    fn recent_provider_copy_uses_recent_and_ascii_count_marker() {
        let snapshot = status_snapshot();
        let normal = status_segments(&snapshot, &[StatusItem::RecentProvider]);
        assert_eq!(normal[0].text, "最近 happy *7");

        let mut unavailable = snapshot.clone();
        unavailable.dominant_provider = ObserverSection::unavailable();
        assert_eq!(
            status_segments(&unavailable, &[StatusItem::RecentProvider])[0].text,
            "最近 不可用"
        );

        let mut empty = snapshot;
        empty.dominant_provider = ObserverSection::empty();
        assert_eq!(
            status_segments(&empty, &[StatusItem::RecentProvider])[0].text,
            "最近 —"
        );
    }

    #[test]
    fn configured_model_route_splits_request_card_but_keeps_statusline_and_detail_semantics() {
        let mut request = request_with_route_counts(1, 0, 0);
        request.model = Some("fable5".to_string());
        request.context_compaction = Some(ObserverContextCompaction {
            mode: "remote".to_string(),
            implementation: "test".to_string(),
            trigger: "test".to_string(),
            reason: "test".to_string(),
            phase: "test".to_string(),
            strategy: "test".to_string(),
        });
        request.requested_reasoning_effort = Some("max".to_string());
        request.configured_model_route = Some(ObserverConfiguredModelRoute {
            source_model: "fable5".to_string(),
            effective_model: "opus4.8".to_string(),
            reasoning_effort: Some("low".to_string()),
            policy_source: "provider".to_string(),
            model_applied: true,
            reasoning_effort_applied: true,
        });

        let lines = request_card_lines(&request, 10, 40);
        let target = "opus4.8-low 压缩·远程";
        assert_eq!(lines.len(), 6);
        assert_eq!(lines[1].kind, RequestCardLineKind::Model);
        assert_eq!(lines[1].text, "Codex / fable5-max →");
        assert_eq!(lines[2].kind, RequestCardLineKind::ModelTarget);
        assert_eq!(
            lines[2].text,
            format!("{}{}", " ".repeat(40 - display_width(target)), target)
        );
        assert!(!lines[1].text.contains("思考"));
        assert!(!lines[2].text.contains("思考"));
        assert_eq!(request_model(&request), "fable5-max→opus4.8-low");
        assert!(detail_lines(&request, 10)
            .iter()
            .any(|line| line == "路由规则  供应商覆盖"));
    }

    #[test]
    fn unchanged_model_route_keeps_one_model_line_with_compact_effort() {
        let mut request = request_with_route_counts(1, 0, 0);
        request.requested_reasoning_effort = Some("max".to_string());
        request.configured_model_route = Some(ObserverConfiguredModelRoute {
            source_model: "gpt-5.6-sol".to_string(),
            effective_model: "gpt-5.6-sol".to_string(),
            reasoning_effort: Some("high".to_string()),
            policy_source: "global".to_string(),
            model_applied: false,
            reasoning_effort_applied: true,
        });

        let lines = request_card_lines(&request, 10, 80);
        assert_eq!(lines.len(), 5);
        assert_eq!(lines[1].kind, RequestCardLineKind::Model);
        assert_eq!(lines[1].text, "Codex / gpt-5.6-sol-high");
        assert!(!lines[1].text.contains("思考"));
    }

    #[test]
    fn model_only_route_inherits_requested_effort_on_both_model_lines() {
        let mut request = request_with_route_counts(1, 0, 0);
        request.model = Some("gpt-5.6-sol".to_string());
        request.requested_reasoning_effort = Some("max".to_string());
        request.configured_model_route = Some(ObserverConfiguredModelRoute {
            source_model: "gpt-5.6-sol".to_string(),
            effective_model: "gpt-5.6-terra".to_string(),
            reasoning_effort: None,
            policy_source: "global".to_string(),
            model_applied: true,
            reasoning_effort_applied: false,
        });

        let lines = request_card_lines(&request, 10, 32);
        assert_eq!(lines[1].text, "Codex / gpt-5.6-sol-max →");
        let target = "gpt-5.6-terra-max";
        assert_eq!(
            lines[2].text,
            format!("{}{}", " ".repeat(32 - display_width(target)), target)
        );
        assert_eq!(request_model(&request), "gpt-5.6-sol-max→gpt-5.6-terra-max");
    }

    #[test]
    fn non_codex_route_keeps_existing_effort_presentation() {
        let mut request = request_with_route_counts(1, 0, 0);
        request.cli_key = "claude".to_string();
        request.model = Some("fable5".to_string());
        request.requested_reasoning_effort = Some("max".to_string());
        request.configured_model_route = Some(ObserverConfiguredModelRoute {
            source_model: "fable5".to_string(),
            effective_model: "opus4.8".to_string(),
            reasoning_effort: Some("low".to_string()),
            policy_source: "provider".to_string(),
            model_applied: true,
            reasoning_effort_applied: true,
        });

        let lines = request_card_lines(&request, 10, 32);
        assert_eq!(lines[1].text, "Claude / fable5 →");
        let target = "opus4.8·low";
        assert_eq!(
            lines[2].text,
            format!("{}{}", " ".repeat(32 - display_width(target)), target)
        );
        assert_eq!(request_model(&request), "fable5→opus4.8·思考low");
    }

    #[test]
    fn route_model_lines_are_grapheme_safe_at_extreme_widths() {
        let mut request = request_with_route_counts(1, 0, 0);
        request.configured_model_route = Some(ObserverConfiguredModelRoute {
            source_model: "超长源模型名称".to_string(),
            effective_model: "目标模型名称非常长".to_string(),
            reasoning_effort: Some("high".to_string()),
            policy_source: "global".to_string(),
            model_applied: true,
            reasoning_effort_applied: true,
        });

        let zero = request_card_lines(&request, 10, 0);
        assert_eq!(zero[1].text, "");
        assert_eq!(zero[2].text, "");
        let one = request_card_lines(&request, 10, 1);
        assert_eq!(one[1].text, "→");
        assert_eq!(one[2].text, "…");
        for width in 0..=8 {
            assert!(request_card_lines(&request, 10, width)
                .iter()
                .all(|line| display_width(&line.text) <= width));
        }
    }

    #[test]
    fn missing_route_falls_back_to_the_original_single_model_line() {
        let mut request = request_with_route_counts(1, 0, 0);
        request.model = Some("原始模型".to_string());
        request.requested_reasoning_effort = Some("max".to_string());

        let lines = request_card_lines(&request, 10, 80);
        assert_eq!(lines.len(), 5);
        assert_eq!(lines[1].text, "Codex / 原始模型-max");
        assert_eq!(request_model(&request), "原始模型-max");
        assert!(lines
            .iter()
            .all(|line| line.kind != RequestCardLineKind::ModelTarget));
    }

    #[test]
    fn invalid_route_fields_fall_back_to_the_original_model() {
        let mut request = request_with_route_counts(1, 0, 0);
        request.model = Some("原始模型".to_string());
        request.configured_model_route = Some(ObserverConfiguredModelRoute {
            source_model: "\u{200b}".to_string(),
            effective_model: "目标模型".to_string(),
            reasoning_effort: Some("high".to_string()),
            policy_source: "global".to_string(),
            model_applied: true,
            reasoning_effort_applied: true,
        });

        let lines = request_card_lines(&request, 10, 80);
        assert_eq!(lines.len(), 5);
        assert_eq!(lines[1].text, "Codex / 原始模型");
        assert_eq!(request_model(&request), "原始模型");
    }
}
