# 设计

ResidentState 增加 macOS 专用 hover 状态机和单实例 WebView。Tray 事件提供锚点，前端 pointer enter/leave 回报共同悬停；离开 180ms 后隐藏。打开时后端生成冻结快照，窗口按独立查询模式只渲染 mini 根组件。
