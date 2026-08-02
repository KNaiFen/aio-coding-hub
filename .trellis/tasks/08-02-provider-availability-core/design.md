# 设计

尝试 JSON 增加向后兼容的 `upstream_sent`。终态日志后台按 trace/provider 聚合后，以独立最佳努力事务写入新表；表只含 trace、CLI、provider、时间和成功标记。统一 Rust 聚合器生成自然边界、原始计数和三态。设置字段仅允许 3/6/12。
