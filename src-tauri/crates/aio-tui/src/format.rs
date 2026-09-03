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

pub fn request_tone(request: &ObserverRequest) -> StatusTone {
    let route = route_presentation(request);
    if request.state == ObserverRequestState::Active {
        StatusTone::Activity
    } else if request.interrupted
        || (route.has_hop_evidence && route.skipped_count > 0 && !route.has_sent_attempt)
    {
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

pub fn route_tone(request: &ObserverRequest) -> StatusTone {
    let route = route_presentation(request);
    if route.provider_switch_count > 0 || route.retry_count > 0 || route.skipped_count > 0 {
        StatusTone::Warning
    } else if route.has_sent_attempt {
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
    let card_timing_ms = match request.state {
        ObserverRequestState::Active => request.duration_ms,
        ObserverRequestState::Terminal => request.ttfb_ms,
    };
    let card_timing = card_timing_ms
        .map(format_duration)
        .unwrap_or_else(|| "—".to_string());
    let (input, output, cache) = request
        .usage
        .as_ref()
        .map(|usage| {
            let cache = match (usage.cache_read_tokens, usage.cache_creation_tokens) {
                (None, None) => "—".to_string(),
                (read, creation) => {
                    format_tokens(read.unwrap_or(0).saturating_add(creation.unwrap_or(0)))
                }
            };
            (
                usage
                    .input_tokens
                    .map(format_tokens)
                    .unwrap_or_else(|| "—".to_string()),
                usage
                    .output_tokens
                    .map(format_tokens)
                    .unwrap_or_else(|| "—".to_string()),
                cache,
            )
        })
        .unwrap_or_else(|| ("—".to_string(), "—".to_string(), "—".to_string()));
    let cost = request
        .cost_usd
        .map(format_cost)
        .unwrap_or_else(|| "—".to_string());
    let route_summary = if let Some(rate) = output_tokens_per_second(request) {
        format!(
            "{}  {}  {}",
            request_card_route_result(request),
            card_timing,
            format_tokens_per_second_short(rate)
        )
    } else if let Some(rate) = estimated_output_tokens_per_second(request) {
        format!(
            "{}  {}  ≈{}",
            request_card_route_result(request),
            card_timing,
            format_tokens_per_second_short(rate)
        )
    } else {
        format!("{}  {}", request_card_route_result(request), card_timing)
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
                &format!("I {} O {} C {} {}", input, output, cache, cost),
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
            truncate_display_with_suffix(
                &format!("{} / {}", cli_label(&request.cli_key), model),
                compaction,
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
            truncate_display_with_suffix(
                &format!("{} / {}", cli_label(&request.cli_key), model),
                compaction,
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
    let target = right_align_display(
        &truncate_display_with_suffix(&target_model, compaction, width),
        width,
    );
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
        && matches!(
            route.policy_source.as_str(),
            "global" | "provider" | "provider_cross"
        )
}

fn truncate_display_with_suffix(lead: &str, suffix: &str, max_width: usize) -> String {
    if suffix.is_empty() {
        return truncate_display(lead, max_width);
    }
    let full = format!("{lead}{suffix}");
    if display_width(&full) <= max_width {
        return full;
    }
    let suffix_width = display_width(suffix);
    if suffix_width > max_width {
        return truncate_display(suffix.trim_start(), max_width);
    }
    format!(
        "{}{}",
        truncate_display(lead, max_width.saturating_sub(suffix_width)),
        suffix
    )
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
        format!(
            "Session复用  {}",
            if request.session_reuse { "是" } else { "否" }
        ),
        format!(
            "输出速率  {}",
            output_tokens_per_second(request)
                .map(format_tokens_per_second_short)
                .or_else(|| {
                    estimated_output_tokens_per_second(request)
                        .map(|rate| format!("≈{}", format_tokens_per_second_short(rate)))
                })
                .unwrap_or_else(|| "—".to_string())
        ),
    ];
    if let Some(route) = request
        .configured_model_route
        .as_ref()
        .filter(|route| configured_model_route_is_valid(route))
    {
        let source = match route.policy_source.as_str() {
            "provider" => "供应商覆盖",
            "global" => "全局",
            "provider_cross" => "跨供应商",
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
            let outcome = if hop.skipped {
                "已跳过/未发送"
            } else if hop.ok {
                "成功"
            } else if request.state == ObserverRequestState::Active
                && hop.status.is_none()
                && hop.error_code.is_none()
            {
                "进行中"
            } else {
                "失败"
            };
            let mut line = format!(
                "{}. {} ×{} {}",
                index + 1,
                hop.provider_name,
                normalize_hop_attempts(hop.attempts),
                outcome
            );
            if let Some(status) = hop.status {
                line.push_str(&format!(" HTTP {status}"));
            }
            if let Some(error_code) = hop.error_code.as_deref() {
                line.push_str(&format!(" {error_code}"));
            }
            lines.push(line);
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
    let route = route_presentation(request);
    if route.has_hop_evidence {
        let mut tokens = route_count_tokens(&route, false);
        if tokens.is_empty() {
            return if route.has_sent_attempt {
                "直连".to_string()
            } else {
                "未上游".to_string()
            };
        }
        if !route.has_sent_attempt {
            tokens.push("未发出上游请求".to_string());
        }
        return tokens.join("/");
    }
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

fn request_card_route_result(request: &ObserverRequest) -> String {
    let route = route_presentation(request);
    if route.has_hop_evidence {
        let tokens = route_count_tokens(&route, true);
        if !tokens.is_empty() {
            return tokens.join("·");
        }
        return if route.has_sent_attempt {
            "直连".to_string()
        } else {
            "未上游".to_string()
        };
    }
    if request.provider_switch_count > 0 {
        if request.retry_count > 0 {
            return format!(
                "切{}/重{}",
                request.provider_switch_count, request.retry_count
            );
        }
        return format!("切{}", request.provider_switch_count);
    }
    if request.retry_count > 0 {
        return format!("重{}", request.retry_count);
    }
    if request.attempt_count > 0 {
        return "直连".to_string();
    }
    "未上游".to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RoutePresentation {
    has_hop_evidence: bool,
    skipped_count: u32,
    sent_attempt_count: u32,
    retry_count: u32,
    provider_switch_count: u32,
    has_sent_attempt: bool,
}

fn route_presentation(request: &ObserverRequest) -> RoutePresentation {
    const COUNT_LIMIT: u32 = 9_999;
    if request.route.is_empty() {
        return RoutePresentation {
            has_hop_evidence: false,
            skipped_count: 0,
            sent_attempt_count: request.attempt_count.min(COUNT_LIMIT),
            retry_count: request.retry_count.min(COUNT_LIMIT),
            provider_switch_count: request.provider_switch_count.min(COUNT_LIMIT),
            has_sent_attempt: request.attempt_count > 0,
        };
    }
    let skipped_count = request
        .route
        .iter()
        .filter(|hop| hop.skipped)
        .count()
        .try_into()
        .unwrap_or(COUNT_LIMIT);
    let sent_attempt_count =
        request
            .route
            .iter()
            .filter(|hop| !hop.skipped)
            .fold(0_u32, |total, hop| {
                total
                    .saturating_add(normalize_hop_attempts(hop.attempts))
                    .min(COUNT_LIMIT)
            });
    RoutePresentation {
        has_hop_evidence: true,
        skipped_count,
        sent_attempt_count,
        retry_count: request.retry_count.min(COUNT_LIMIT),
        provider_switch_count: request.provider_switch_count.min(COUNT_LIMIT),
        has_sent_attempt: sent_attempt_count > 0,
    }
}

fn normalize_hop_attempts(attempts: u32) -> u32 {
    attempts.clamp(1, 9_999)
}

fn route_count_tokens(route: &RoutePresentation, compact: bool) -> Vec<String> {
    let mut tokens = Vec::new();
    if route.provider_switch_count > 0 {
        tokens.push(if compact {
            format!("切{}", route.provider_switch_count)
        } else {
            format!("切换{}", route.provider_switch_count)
        });
    }
    if route.skipped_count > 0 {
        tokens.push(if compact {
            format!("跳{}", route.skipped_count)
        } else {
            format!("跳过{}", route.skipped_count)
        });
    }
    if route.retry_count > 0 {
        tokens.push(if compact {
            format!("重{}", route.retry_count)
        } else {
            format!("重试{}", route.retry_count)
        });
    }
    let has_route_signal = !tokens.is_empty();
    if route.sent_attempt_count > 0 && (has_route_signal || route.sent_attempt_count > 1) {
        tokens.push(if compact {
            format!("请{}", route.sent_attempt_count)
        } else {
            format!("请求{}", route.sent_attempt_count)
        });
    }
    tokens
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

fn estimated_output_tokens_per_second(request: &ObserverRequest) -> Option<f64> {
    if output_tokens_per_second(request).is_some()
        || request.state != ObserverRequestState::Terminal
        || request.error_code.is_some()
        || !request
            .status
            .is_some_and(|status| (200..300).contains(&status))
    {
        return None;
    }
    let output_tokens = request.usage.as_ref()?.output_tokens?;
    let estimated_final_upstream_attempt_duration_ms =
        request.estimated_final_upstream_attempt_duration_ms?;
    if output_tokens <= 0 || estimated_final_upstream_attempt_duration_ms <= 0 {
        return None;
    }
    let rate =
        output_tokens as f64 / (estimated_final_upstream_attempt_duration_ms as f64 / 1_000.0);
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
        ObserverGatewayStatus, ObserverPreferredProvider, ObserverRequestUsage, ObserverRouteHop,
        ObserverSection, ObserverTodayUsage, OBSERVER_PROTOCOL_VERSION,
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
            estimated_final_upstream_attempt_duration_ms: None,
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

    fn route_hop(
        provider_name: &str,
        attempts: u32,
        skipped: bool,
        ok: bool,
        status: Option<i64>,
        error_code: Option<&str>,
    ) -> ObserverRouteHop {
        ObserverRouteHop {
            provider_name: provider_name.to_string(),
            attempts,
            skipped,
            ok,
            status,
            error_code: error_code.map(str::to_string),
        }
    }

    fn compaction(mode: &str) -> ObserverContextCompaction {
        ObserverContextCompaction {
            mode: mode.to_string(),
            implementation: "test".to_string(),
            trigger: "test".to_string(),
            reason: "test".to_string(),
            phase: "test".to_string(),
            strategy: "test".to_string(),
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
    fn request_card_selects_timing_by_state_and_keeps_output_rate_terminal_only() {
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
        assert_eq!(lines[3].text, "直连  500ms  100.0 t/s");
        assert!(request_card_lines(&request, 60_001, 15)[2]
            .text
            .starts_with("INPUT 大春"));

        request.ttfb_ms = None;
        assert_eq!(
            request_card_lines(&request, 60_001, 80)[3].text,
            "直连  —  100.0 t/s"
        );

        request.state = ObserverRequestState::Active;
        assert_eq!(
            request_card_lines(&request, 60_001, 80)[3].text,
            "直连  2.0s"
        );

        request.ttfb_ms = Some(500);
        assert_eq!(
            request_card_lines(&request, 60_001, 80)[3].text,
            "直连  2.0s"
        );
    }

    #[test]
    fn request_card_compacts_route_counts_without_changing_shared_views() {
        let mut request = request_with_route_counts(5, 3, 1);
        request.duration_ms = Some(2_000);
        request.ttfb_ms = Some(500);

        assert_eq!(
            request_card_lines(&request, 60_001, 80)[3].text,
            "切1/重3  500ms"
        );
        assert_eq!(request_card_route_result(&request), "切1/重3");
        assert_eq!(route_result(&request), "切换1/重试3");

        let details = detail_lines(&request, 60_001);
        assert!(details.iter().any(|line| line == "路由  切换1/重试3"));
        assert!(details.iter().any(|line| line == "耗时  2.0s"));
        assert!(details.iter().any(|line| line == "首字  500ms"));

        assert_eq!(
            request_card_route_result(&request_with_route_counts(2, 0, 1)),
            "切1"
        );
        assert_eq!(
            request_card_route_result(&request_with_route_counts(4, 3, 0)),
            "重3"
        );
        assert_eq!(
            request_card_route_result(&request_with_route_counts(1, 0, 0)),
            "直连"
        );
        assert_eq!(
            request_card_route_result(&request_with_route_counts(0, 0, 0)),
            "未上游"
        );
    }

    #[test]
    fn structured_route_summaries_keep_skip_retry_switch_and_sent_counts() {
        let mut skipped_only = request_with_route_counts(1, 0, 0);
        skipped_only.route = vec![route_hop("候选 A", 0, true, false, None, None)];

        assert_eq!(request_card_route_result(&skipped_only), "跳1");
        assert_eq!(route_result(&skipped_only), "跳过1/未发出上游请求");
        assert_eq!(route_tone(&skipped_only), StatusTone::Warning);
        assert_eq!(request_tone(&skipped_only), StatusTone::Warning);
        let skipped_detail = detail_lines(&skipped_only, 10);
        assert!(skipped_detail
            .iter()
            .any(|line| line == "1. 候选 A ×1 已跳过/未发送"));
        assert!(!skipped_detail.iter().any(|line| line.contains("直连")));

        let mut mixed = request_with_route_counts(3, 1, 1);
        mixed.route = vec![
            route_hop("候选 A", 1, true, false, None, None),
            route_hop("供应商 B", 2, false, false, Some(500), Some("UPSTREAM")),
            route_hop("供应商 C", 1, false, true, Some(200), None),
        ];
        assert_eq!(request_card_route_result(&mixed), "切1·跳1·重1·请3");
        assert_eq!(route_result(&mixed), "切换1/跳过1/重试1/请求3");
        let mixed_detail = detail_lines(&mixed, 10);
        assert!(mixed_detail
            .iter()
            .any(|line| line == "2. 供应商 B ×2 失败 HTTP 500 UPSTREAM"));
        assert!(mixed_detail
            .iter()
            .any(|line| line == "3. 供应商 C ×1 成功 HTTP 200"));

        let mut active = request_with_route_counts(1, 0, 0);
        active.state = ObserverRequestState::Active;
        active.status = None;
        active.route = vec![route_hop("进行中供应商", 1, false, false, None, None)];
        assert!(detail_lines(&active, 10)
            .iter()
            .any(|line| line == "1. 进行中供应商 ×1 进行中"));
    }

    #[test]
    fn request_card_cache_keeps_unknown_distinct_from_zero_and_detail_keeps_evidence() {
        let mut request = request_with_route_counts(1, 0, 0);
        request.session_reuse = true;
        request.final_upstream_attempt_duration_ms = Some(2_000);
        request.final_upstream_attempt_timing_version = 1;
        request.usage = Some(ObserverRequestUsage {
            input_tokens: None,
            output_tokens: Some(200),
            total_tokens: None,
            cache_read_tokens: None,
            cache_creation_tokens: None,
        });

        assert_eq!(
            request_card_lines(&request, 10, 80)[4].text,
            "I — O 200 C — —"
        );
        let details = detail_lines(&request, 10);
        assert!(details.iter().any(|line| line == "Session复用  是"));
        assert!(details.iter().any(|line| line == "输出速率  100.0 t/s"));

        request.usage.as_mut().expect("usage").cache_read_tokens = Some(7);
        assert_eq!(
            request_card_lines(&request, 10, 80)[4].text,
            "I — O 200 C 7 —"
        );
        request.usage.as_mut().expect("usage").cache_creation_tokens = Some(2);
        assert_eq!(
            request_card_lines(&request, 10, 80)[4].text,
            "I — O 200 C 9 —"
        );
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
    fn detail_shows_same_boundary_estimated_output_rate() {
        let mut request = request_with_route_counts(2, 1, 1);
        request.estimated_final_upstream_attempt_duration_ms = Some(20_000);
        request.usage = Some(ObserverRequestUsage {
            input_tokens: None,
            output_tokens: Some(248),
            total_tokens: None,
            cache_read_tokens: None,
            cache_creation_tokens: None,
        });

        assert_eq!(output_tokens_per_second(&request), None);
        assert_eq!(estimated_output_tokens_per_second(&request), Some(12.4));
        assert!(detail_lines(&request, 10)
            .iter()
            .any(|line| line == "输出速率  ≈12.4 t/s"));
        assert!(request_card_lines(&request, 10, 80)[3].text.contains('≈'));

        request.final_upstream_attempt_duration_ms = Some(10_000);
        request.final_upstream_attempt_timing_version = 1;
        assert_eq!(output_tokens_per_second(&request), Some(24.8));
        assert_eq!(estimated_output_tokens_per_second(&request), None);
        assert!(detail_lines(&request, 10)
            .iter()
            .any(|line| line == "输出速率  24.8 t/s"));

        request.final_upstream_attempt_duration_ms = None;
        request.final_upstream_attempt_timing_version = 0;
        request.status = Some(500);
        assert_eq!(estimated_output_tokens_per_second(&request), None);
        request.status = Some(200);
        request.estimated_final_upstream_attempt_duration_ms = None;
        assert_eq!(estimated_output_tokens_per_second(&request), None);
        request.estimated_final_upstream_attempt_duration_ms = Some(20_000);
        request.usage.as_mut().expect("usage").output_tokens = None;
        assert_eq!(estimated_output_tokens_per_second(&request), None);
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
    fn compaction_suffix_remains_visible_across_all_model_card_paths() {
        for (mode, label) in [("local", "压缩·本地"), ("remote", "压缩·远程")] {
            let mut unrouted = request_with_route_counts(1, 0, 0);
            unrouted.model = Some("gpt-5.6-ultra-long-model-name".to_string());
            unrouted.requested_reasoning_effort = Some("ultra".to_string());
            unrouted.context_compaction = Some(compaction(mode));

            let mut unchanged = unrouted.clone();
            unchanged.configured_model_route = Some(ObserverConfiguredModelRoute {
                source_model: "gpt-5.6-ultra-long-model-name".to_string(),
                effective_model: "gpt-5.6-ultra-long-model-name".to_string(),
                reasoning_effort: Some("max".to_string()),
                policy_source: "global".to_string(),
                model_applied: false,
                reasoning_effort_applied: true,
            });

            let mut changed = unrouted.clone();
            changed.configured_model_route = Some(ObserverConfiguredModelRoute {
                source_model: "gpt-5.6-ultra-long-source".to_string(),
                effective_model: "gpt-5.6-ultra-long-target".to_string(),
                reasoning_effort: Some("max".to_string()),
                policy_source: "provider".to_string(),
                model_applied: true,
                reasoning_effort_applied: true,
            });

            for request in [&unrouted, &unchanged, &changed] {
                for width in [0, 1, 24, 31, 32, 80] {
                    let lines = request_card_lines(request, 10, width);
                    assert!(lines.iter().all(|line| display_width(&line.text) <= width));
                    if width >= 24 {
                        assert!(lines.iter().any(|line| line.text.contains(label)));
                    }
                }
            }
        }
    }

    #[test]
    fn provider_cross_route_uses_shared_model_and_policy_presentation() {
        let mut request = request_with_route_counts(1, 0, 0);
        request.requested_reasoning_effort = Some("high".to_string());
        request.configured_model_route = Some(ObserverConfiguredModelRoute {
            source_model: "gpt-5.6-sol".to_string(),
            effective_model: "gpt-5.6-terra".to_string(),
            reasoning_effort: Some("max".to_string()),
            policy_source: "provider_cross".to_string(),
            model_applied: true,
            reasoning_effort_applied: true,
        });

        assert_eq!(
            request_model(&request),
            "gpt-5.6-sol-high→gpt-5.6-terra-max"
        );
        assert_eq!(request_card_lines(&request, 10, 80).len(), 6);
        assert!(detail_lines(&request, 10)
            .iter()
            .any(|line| line == "路由规则  跨供应商"));

        request.state = ObserverRequestState::Active;
        request.status = None;
        assert_eq!(
            request_model(&request),
            "gpt-5.6-sol-high→gpt-5.6-terra-max"
        );
        assert_eq!(request_card_lines(&request, 10, 80).len(), 6);
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

    #[test]
    fn future_route_policy_falls_open_without_a_target_or_policy_label() {
        let mut request = request_with_route_counts(1, 0, 0);
        request.model = Some("原始模型".to_string());
        request.configured_model_route = Some(ObserverConfiguredModelRoute {
            source_model: "原始模型".to_string(),
            effective_model: "未来目标".to_string(),
            reasoning_effort: None,
            policy_source: "provider_future".to_string(),
            model_applied: true,
            reasoning_effort_applied: false,
        });

        let lines = request_card_lines(&request, 10, 80);
        assert_eq!(lines.len(), 5);
        assert_eq!(lines[1].text, "Codex / 原始模型");
        assert!(!detail_lines(&request, 10)
            .iter()
            .any(|line| line.starts_with("路由规则")));
    }
}
