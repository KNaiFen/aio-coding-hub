# 项目规则统一采用 GKD：合并交付

- 日期：2026-09-08；revision：r2-delivery；状态：用户已授权执行。
- 路线：direct-main 复核既有交付证据，由一个 gkd_closeout 完成归档、PR、CI 等待、合并和清理；没有新的实施任务。
- 用户先要求清理无用的 gkd-project-rules/worktrees，现明确要求该分支未合并则合并，有冲突告知。此授权包含必要的任务分支推送、PR 创建、正常门禁后 squash 合并；交付完成后清理已无独有成果的本任务工作树和分支。

## 目标与当前证据

把 chore/gkd-project-rules 中已经实施、检查和审查通过的规则精简合入 origin/main。原任务归档位于上一 revision，历史正文保持原观察时点，不改写当时的授权。

- source：a04f49597793d4f421e466208bcb9310da82c78e；原实施基线：a35f2aa4ea469f6e4066582b2e969f1ec44fca2e。
- 已 fetch 的 origin/main：42b64aa646e19ce6b1f3bf6a08219043ea960dd4；本地 main：193767510ef647193ce5f16390bc1f663c3dffb0。
- 来源工作树干净，未找到此 head 分支的远端分支或 PR；git cherry 仍列出 source 独有提交。
- 合并预演无冲突；任务差异限于既有 24 个文件，业务代码、包配置和 .github 工作流没有进入任务差异。

## 范围、验证与终点

复用 r1 的实施和独立验收结果，不重跑实现测试。新增交付记录只保存本轮必要事实。自动 PR 检查通过后，以已验证 head 绑定 squash 合并；再尝试将 origin/main 普通合并到本地 main。成果与记录进入远端 main 且来源无新增待保留内容后，清理本任务工作树和分支；保留本地 main 的独有历史及其他任务归档。

不运行依赖安装、package manager、开发服务器、lint、类型检查、测试、构建、Cargo、Tauri、签名或打包。允许的本地验证限普通只读 Git/gh、目录状态和必要归档差异检查；远端自动 CI 承担实现验证。
