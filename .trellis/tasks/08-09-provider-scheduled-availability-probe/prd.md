# 供应商定时可用性测试

## Goal

为每个供应商增加默认关闭的后台定时 probe，并让它与全部同义手动入口共享实现和状态条观测。

## Requirements

- 编辑器配置开关和 `1..=1440` 分钟间隔，默认 false/10；只调度 enabled Provider。
- 以本地日 00:00 为锚点，下一边界后 5s + 稳定 0..3s 错峰，并发上限 4；不补当前或历史周期。
- 桌面、Observer/TUI 和 scheduler 复用同一 probe/coordinator；Base URL Ping 不属于可用性观测。
- 真实请求与主动 probe 等权进入现有观测表，不向 UI 暴露来源。
- 实施与测试代码完成后等待父任务统一验证。

## Acceptance Criteria

- [ ] 关闭时无额外网络请求；页面关闭后定时任务仍运行。
- [ ] 启动/唤醒/配置变更不补跑，稳定 trace ID 保证每 Provider/周期最多一次。
- [ ] 配置失效、禁用、删除或内部错误不写入过期/伪失败观测，日志不含凭据。
