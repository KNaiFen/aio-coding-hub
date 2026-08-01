use aio_observer_protocol::CliScope;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Logs,
    Status { once: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
                mode = Mode::Status { once: false };
                mode_seen = true;
            }
            "--once" => match mode {
                Mode::Status { .. } => mode = Mode::Status { once: true },
                Mode::Logs => return Err("--once 只能与 status 一起使用".to_string()),
            },
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
  aio-tui status [--once] [--cli <scope>]\n\n\
scope：claude | codex | grok | gemini | all（默认 codex）\n\n\
命令：\n\
  logs      可滚动的活动请求与最近 50 条记录（默认）\n\
  status    持续更新的一行状态栏；窄窗口自动换行\n\
  --once    输出一次纯文本状态后退出\n"
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
        assert_eq!(args.mode, Mode::Status { once: true });
        assert_eq!(args.scope, CliScope::All);
    }

    #[test]
    fn rejects_once_in_logs_mode() {
        assert!(parse_from(["--once"]).is_err());
    }
}
