# 技术设计

三个修复保持独立实现和提交，但共享同一分支、PR、版本号与发布候选。父任务不引入额外产品接口，只负责跨任务一致性、版本同步、CI 漂移处理、合并和发布证据。

发布遵守 promotion contract：版本变更随 PR 进入 main；使用该合并 SHA 的唯一成功 release candidate；候选成功后再创建 canonical tag；release workflow 只晋升既有制品，不重建。

回滚以原子提交为边界。合并前可逐提交修正；发布后如需回退，创建新的补丁版本，不覆盖既有 tag 或 Release 资产。
