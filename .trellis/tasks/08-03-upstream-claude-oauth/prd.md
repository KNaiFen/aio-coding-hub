# 验证并适配 Claude OAuth 入口

## Goal

候选移植 7bd1812f；仅在隔离账号完整 OAuth 验证通过后合并。

## Requirements

- 内置 Claude OAuth client 使用 `https://claude.ai/oauth/authorize`，token endpoint 保持不变。
- 内置 Anthropic exchange/refresh 使用专用 token UA；自定义 client 环境变量保持当前入口和请求身份。
- 保留响应大小限制、错误脱敏、timeout/invalid_grant 分类和 secret 不落日志约束。
- 只有真实 Claude Pro/Max 隔离账号完成完整登录与刷新验证后才允许合并。

## Acceptance Criteria

- [ ] 内置与自定义 client 的授权 URL 和 token UA 分流有单元测试。
- [ ] 浏览器回调、exchange、手动刷新、后台/请求前刷新和 401 刷新成功。
- [ ] 失败响应、超时和 invalid_grant 不泄露远端 body 或 token。
- [ ] 无法完成真实验证时任务保持候选状态，不进入发布。
- [ ] 变更可追溯到 `7bd1812f9502670dd7536f251fbaf8fcc27966bd`。

## Notes

- 上游固定 `axios/1.13.6` 只是兼容指纹，不应扩散到其他 OAuth Provider。
