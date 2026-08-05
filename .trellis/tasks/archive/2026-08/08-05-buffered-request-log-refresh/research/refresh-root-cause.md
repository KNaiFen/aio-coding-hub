# 请求日志刷新根因

- complete 事件在第一页且前台时以约 2 秒窗口合并调用 `refreshSnapshot`。
- `refreshSnapshot` 清空 snapshot id 并增加 revision，新 query 虽有 placeholder data，feed 却主动返回空数组。
- 列表高度因此归零，浏览器钳制 scrollTop；新快照又按 created time 倒序在头部插入，用户丢失阅读位置。
- 后端 snapshot membership 本身稳定，修复应限定在前端展示与实时刷新协调。
