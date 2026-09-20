import type { PluginHook } from "../../services/plugins";

// Public beforeCommit behavior, shared by install preview and installed details.
export function PluginResponseCommitNotice({
  hooks,
  capabilities = [],
}: {
  hooks: readonly PluginHook[];
  capabilities?: readonly string[];
}) {
  const hook = hooks.find((item) => item.name === "gateway.response.beforeCommit");
  if (!hook) return null;
  return (
    <div className="space-y-1 rounded-md border border-warning/30 bg-warning/10 px-3 py-2 text-sm text-foreground">
      <div className="font-medium">完整响应校验</div>
      {hook.match ? (
        <div className="break-words font-mono text-xs text-muted-foreground">
          作用范围：{hook.match.cliKeys.join(", ")} · {hook.match.methods.join(", ")} ·{" "}
          {hook.match.paths.join(", ")}
        </div>
      ) : null}
      <p>命中范围的请求会等待上游完整返回并通过校验后才显示内容，首字等待会更长。</p>
      {capabilities.includes("gateway.provider.switch") ? (
        <p>
          插件申请换家时，会丢弃当前响应，并在可安全重放时按当前会话的合法供应商顺序尝试下一家；其他会话不受该拒绝影响。
        </p>
      ) : null}
      <p className="text-xs text-muted-foreground">
        启停影响后续请求；在途请求仍须完成已确定的校验，无法校验时会明确失败。
      </p>
    </div>
  );
}
