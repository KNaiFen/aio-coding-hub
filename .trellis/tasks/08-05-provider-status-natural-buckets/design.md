# 技术设计

`timelines` 使用 `bucket_ms = range / bucket_count`。删除 desktop 36 格强制复用 TUI 12 格对齐宽度的特判，使 `alignment_ms = bucket_ms`。当前期间起点随调用方自然粒度变化，已有“当前最后一格取最后请求状态”规则无需改写。

12、18、36 格共享同一算法与排序；接口、DTO 和前端格数均不变。测试分别锁定三种窗口边界，不再断言每个 TUI 格等于三个 desktop 格。
