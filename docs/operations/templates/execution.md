# GKD 执行交接：{{任务名称}}

> 由 canonical bundle 生成；不要手写或改动机器字段。

- 任务目录：`tasks/{{task}}/`
- offer ID：`{{bundle 输出}}`
- 登记 worktree：`{{bundle 输出}}`
- base / planning digest：`{{bundle 输出}}`
- 执行角色：`gkd_executor`
- 允许范围：{{来自 requirements/plan}}
- 停止条件：{{policy、角色、路径、base、bundle 或 head 漂移；材料性变化；阻塞}}

开工首个写操作必须是 `gkd-task claim`。claim 成功前不得修改仓库。
