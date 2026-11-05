# CoRide-API 待修复问题列表

> 最后更新: 2026-11-06
> 状态: **当前无已知未修复缺陷**

## 说明

2026-11-06 的全量审查（见 `docs/review-2026-11-06.md`）共发现 7 项 P0（阻断级）与
15 项 P1（功能失效级）缺陷，已全部修复并通过自动化测试，记录归档在
`buglist-completed.md` 的「十五、2026-11-06 P0/P1 全量修复」章节。

## 剩余技术债（非缺陷，按需处理）

| 编号 | 说明 | 严重程度 | 建议 |
|------|------|----------|------|
| TD-001 | `admin_routes.rs` 约 2500 行，可拆分为子模块 | LOW | 后续按 admin 域拆分 |
| TD-002 | `models.rs` 约 1600 行，可按实体拆分 | LOW | 与 TD-001 一起做 |
| TD-003 | `Channel` 结构体仍使用 `r#type` | LOW | 需要同步迁移 SQL 别名，收益低 |
| TD-004 | 渠道配额超限时按"估算 Token"预检，可能超额一次请求 | LOW | Token 无法预知，属预期行为 |
| TD-005 | 表名单复数混用（`channels` / `models` / `quotas`） | LOW | 破坏性变更，不建议实施 |
| TD-006 | 端到端测试依赖本机后端二进制，未接入 CI | MEDIUM | 可在 GitHub Actions 里 `cargo build` 后运行 `python tests/run_tests.py` |

## 测试入口

```bash
cd backend && cargo test          # 80 个单元测试
cd backend && cargo build         # 集成测试需要 debug 二进制
python tests/run_tests.py         # 69 个集成测试（P0 + P1 + 回归 + 老库升级）
```
