# Codex Model Consistency

普通 Extension Host 插件，使用公共 `gateway.response.beforeCommit` 和
`gateway.provider.switch` 能力，无宿主内置的模型规则或专用开关。
需要支持该公共 hook 的 AIO 版本；旧版即使满足应用版本范围，也会在清单预检拒绝未知 hook。

在仓库根目录验证并打包：

```sh
pnpm --filter create-aio-plugin cli validate --strict examples/plugins/codex-model-consistency
pnpm --filter create-aio-plugin cli pack examples/plugins/codex-model-consistency
```

在「插件」页面选择「导入 .aio-plugin」，选取命令输出的 .aio-plugin 文件，完成预检后启用。
`extension.cjs` 就是交付入口，不需要构建，测试直接运行该入口。

仅覆盖 Codex 原生 HTTP `POST /responses` 和 `/v1/responses` 的 JSON/SSE。
完整读取后才允许向客户端交付，首字因此需要等待上游完成；未命中的请求不受影响。
解码后的完整正文上限为 2 MiB，整次提交前校验最多等待 120 秒（已配置的非流式总超时更短时取更短值），
整个应用共享最多 2 个完整响应准入名额；超出容量、并发或等待预算时明确失败，不会退回未经校验的流式交付。
所有协议 `model` 必须精确等于当前 attempt 最终出站模型，至少有一个有效声明；
别名或版本名称也不等价。正常重复同名事件通过，任一冲突、非法声明、歧义键、
失败或不完整终态都会拒绝。文本、工具参数中的业务 `model` 不参与比较。

拒绝后只沿当前会话本次请求的合法候选换家，不改变其他会话的供应商健康或绑定。
私有续接缺少完整历史时网关明确失败，不删除上一家 response ID 后有损重试。
启停对后续请求生效；已经开始的必需校验不会因关闭插件而绕过。

本插件检查上游**声明**，无法证明上游实际执行的模型身份或同名模型质量。
没有宽松模式、别名映射或配置项。
