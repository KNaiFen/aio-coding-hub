use crate::config::{parse_status_items, StatusItem};
use aio_observer_protocol::CliScope;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    Logs,
    Status {
        once: bool,
        items: Option<Vec<StatusItem>>,
    },
    Statusline,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Args {
    pub mode: Mode,
    pub scope: CliScope,
}

pub enum ParseOutcome {
    Run(Args),
    Help,
    Version,
}

pub fn parse() -> Result<ParseOutcome, String> {
    parse_from(std::env::args().skip(1))
}

fn parse_from<I, S>(values: I) -> Result<ParseOutcome, String>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let values = values.into_iter().map(Into::into).collect::<Vec<_>>();
    if values
        .iter()
        .any(|value| matches!(value.as_str(), "-h" | "--help"))
    {
        return Ok(ParseOutcome::Help);
    }
    if values
        .iter()
        .any(|value| matches!(value.as_str(), "-V" | "--version"))
    {
        return Ok(ParseOutcome::Version);
    }

    let mut mode = Mode::Logs;
    let mut scope = CliScope::Codex;
    let mut mode_seen = false;
    let mut index = 0;
    while index < values.len() {
        match values[index].as_str() {
            "logs" => {
                if mode_seen {
                    return Err("只能指定一个模式".to_string());
                }
                mode = Mode::Logs;
                mode_seen = true;
            }
            "status" => {
                if mode_seen {
                    return Err("只能指定一个模式".to_string());
                }
                mode = Mode::Status {
                    once: false,
                    items: None,
                };
                mode_seen = true;
            }
            "statusline" => {
                if mode_seen {
                    return Err("只能指定一个模式".to_string());
                }
                mode = Mode::Statusline;
                mode_seen = true;
            }
            "--once" => match mode {
                Mode::Status { items, .. } => mode = Mode::Status { once: true, items },
                Mode::Logs | Mode::Statusline => {
                    return Err("--once 只能与 status 一起使用".to_string())
                }
            },
            "--items" => {
                index += 1;
                let value = values
                    .get(index)
                    .ok_or_else(|| "--items 缺少值".to_string())?;
                let parsed = parse_status_items(value)?;
                match mode {
                    Mode::Status { once, .. } => {
                        mode = Mode::Status {
                            once,
                            items: Some(parsed),
                        }
                    }
                    Mode::Logs | Mode::Statusline => {
                        return Err("--items 只能与 status 一起使用".to_string())
                    }
                }
            }
            "--cli" => {
                index += 1;
                let value = values
                    .get(index)
                    .ok_or_else(|| "--cli 缺少值".to_string())?;
                scope = CliScope::parse(value)
                    .ok_or_else(|| "--cli 必须是 claude、codex、grok、gemini 或 all".to_string())?;
            }
            value if value.starts_with('-') => {
                return Err(format!("未知选项：{value}"));
            }
            value => return Err(format!("未知模式：{value}")),
        }
        index += 1;
    }

    Ok(ParseOutcome::Run(Args { mode, scope }))
}

pub fn help() -> &'static str {
    "AIO Coding Hub 终端信息面板\n\n\
用法：\n\
  aio-tui [logs] [--cli <scope>]\n\
  aio-tui status [--once] [--items <keys>] [--cli <scope>]\n\
  aio-tui statusline [--cli <scope>]\n\n\
scope：claude | codex | grok | gemini | all（默认 codex）\n\n\
命令：\n\
  logs      可滚动的活动请求与最近 50 条记录（默认）\n\
  status    持续更新的一行状态栏；窄窗口自动换行\n\
  statusline 交互选择、排序并保存状态栏项目\n\n\
选项：\n\
  --once          输出一次无颜色纯文本状态后退出\n\
  --items <keys>  临时选择状态栏项目，逗号分隔，不修改保存的配置\n\n\
状态栏项目：\n\
  gateway, scope, preferred-provider, last-request, last-status,\n\
  last-provider, last-route, last-model, last-folder, last-duration,\n\
  last-ttfb, last-cost, recent-provider, concurrency, today-cost,\n\
  today-tokens, app-version\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_codex_logs() {
        let ParseOutcome::Run(args) = parse_from(Vec::<String>::new()).expect("parse") else {
            panic!("expected run");
        };
        assert_eq!(args.mode, Mode::Logs);
        assert_eq!(args.scope, CliScope::Codex);
    }

    #[test]
    fn parses_status_once_and_all_scope() {
        let ParseOutcome::Run(args) =
            parse_from(["status", "--once", "--cli", "all"]).expect("parse")
        else {
            panic!("expected run");
        };
        assert_eq!(
            args.mode,
            Mode::Status {
                once: true,
                items: None,
            }
        );
        assert_eq!(args.scope, CliScope::All);
    }

    #[test]
    fn parses_temporary_status_items_in_order() {
        let ParseOutcome::Run(args) =
            parse_from(["status", "--items", "gateway,last-model,today-cost"])
                .expect("parse")
        else {
            panic!("expected run");
        };
        assert_eq!(
            args.mode,
            Mode::Status {
                once: false,
                items: Some(vec![
                    StatusItem::Gateway,
                    StatusItem::LastModel,
                    StatusItem::TodayCost,
                ]),
            }
        );
    }

    #[test]
    fn parses_statusline_configurator() {
        let ParseOutcome::Run(args) = parse_from(["statusline", "--cli", "gemini"])
            .expect("parse")
        else {
            panic!("expected run");
        };
        assert_eq!(args.mode, Mode::Statusline);
        assert_eq!(args.scope, CliScope::Gemini);
    }

    #[test]
    fn rejects_once_in_logs_mode() {
        assert!(parse_from(["--once"]).is_err());
    }

    #[test]
    fn rejects_unknown_or_misplaced_status_items() {
        assert!(parse_from(["status", "--items", "future"]).is_err());
        assert!(parse_from(["logs", "--items", "gateway"]).is_err());
        assert!(parse_from(["statusline", "--items", "gateway"]).is_err());
    }
}
