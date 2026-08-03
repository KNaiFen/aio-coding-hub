# 隐藏 About 未知元数据

## Goal

选择性移植 de09d645，隐藏未知 Bundle 与运行模式字段。

## Requirements

- Bundle 未知时不显示 `Bundle —`。
- 运行模式未知时不显示 `运行模式 unknown`。
- 已知 desktop/portable/bundle 状态及 portable action 保持现有行为。

## Acceptance Criteria

- [ ] known bundle + desktop、unknown bundle、unknown run mode 和 portable 四种状态均有组件测试。
- [ ] 变更仅涉及 About 展示，可追溯到 `de09d64509a1d389e4da57c79317612b66cf02ea`。

## Notes

- 不修改版本、更新、portable 路径或发布矩阵。
