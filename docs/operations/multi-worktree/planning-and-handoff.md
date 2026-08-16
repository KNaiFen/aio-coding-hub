# 规划与交接

本页只供 main 在规划、创建 worktree 和启动独立执行 session 时读取。共享事实边界见[主入口](../multi-worktree-delivery.md)。

## 规划门

1. 读取 `PENDING.md`、相关代码和现行规范，查清能由仓库回答的技术事实。
2. 把用户决定、范围、非目标、AC、停止条件和实施授权写入 `prd.md`；材料性未决问题未关闭时不施工。
3. 复杂任务把设计和步骤分别写入 `design.md`、`implement.md`；`execution.md` 只引用它们并保留执行者不能漏看的差量。
4. 检查活动任务的文件、接口、数据和合并顺序。不同文件不代表没有语义冲突。
5. 规划材料先提交。记录完整规划提交 SHA，不从聊天重建方案。

## 创建 Worktree

创建前 `fetch origin`，确认 main 检出干净并与 `origin/main` 同步。任务必须从记录的完整 `origin/main` SHA 派生，使用明确的 sibling 路径和 `task/*` 分支；不要从脏检出或模糊引用派生。

创建后，在任务 worktree 中用协调命令登记事实：

```bash
python3 .trellis/scripts/task.py delegate <task> \
  --worktree "$(pwd -P)" \
  --branch <branch> \
  --base-sha <full-origin-main-sha> \
  --planning-commit <full-planning-sha> \
  --writer <execution-session>
git add .trellis/tasks/<task>/task.json
git commit -m "chore(workflow): 登记任务协调状态"
python3 .trellis/scripts/task.py start <task>
git add .trellis/tasks/<task>/task.json
git commit -m "chore(workflow): 启动任务执行"
```

`delegate` 登记已有 worktree，不创建或删除 worktree。命令会核对规范化路径、当前分支、完整 SHA、base 关系和规划提交祖先关系；失败时修正事实，不手改 JSON 绕过。

## execution.md

使用[施工入口模板](../templates/execution.md)，只写任务特有内容：

- 权威材料和现行规范的直接路径。
- 已锁定决定和允许执行者自行选择的实现细节。
- 必须完成、允许修改、范围外和并行冲突。
- AC ID 对应的验证入口。
- 本地允许检查、云端检查和人工验证责任。
- 任务特有的停止条件。

路径、分支、writer、base 和规划提交由 `task.py status` 输出，不在 `execution.md` 重复维护。比较当前目录时使用 `pwd -P`，避免逻辑路径和物理路径不同造成误判。

## 生成交接

协调提交后必须保持 worktree 干净：

```bash
python3 .trellis/scripts/task.py doctor <task>
python3 .trellis/scripts/task.py handoff <task>
```

`handoff` 输出可直接发送的新窗口 Prompt。新窗口必须以登记 worktree 为 primary folder，显式调用 `$aio-trellis-execute`，才能稳定发现仓库级 skill。

交接前最后确认：任务处于 `implementing`、唯一写者正确、执行入口存在、依赖和冲突已说明。执行 session 开始后，main 不写该 worktree，直到它明确暂停。

