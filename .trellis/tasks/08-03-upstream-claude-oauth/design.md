# Claude OAuth 适配设计

- `ClaudeOAuthProvider::new` 根据 client 环境变量是否存在区分内置和自定义模式。
- 内置模式配置 `claude.ai` authorize URL 和可选 token request UA；自定义模式沿用当前 platform URL 且不强制 UA。
- 将 token request identity 作为 OAuth endpoint/request metadata 显式传入 exchange 与 refresh，禁止通过全局 HTTP client偷渡。
- UPA-005 与本项保持独立，候选项回滚不影响状态缓存修复。
