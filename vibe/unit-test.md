# CoRide-API 单元测试清单

> 本清单供 AI Agent 执行单元测试使用
> 最后更新: 2026-04-18

---

## 测试框架约定

- 使用 Rust 内置 `#[cfg(test)]` + `#[test]` 属性
- 每个模块的测试写在对应文件的 `mod tests { ... }` 中
- 需要 mock 的依赖使用 `mockall` crate 或手动构造测试数据
- 数据库相关测试使用 SQLite 内存数据库 `:memory:`
- 测试命名规范: `test_<function_name>_<scenario>`

---

## 一、纯函数测试（优先级最高，无外部依赖）

### 1.1 Token 估算 (`utils/token_counter.rs`)

> 已有 3 个基础测试，需补充以下用例

| # | 测试名称 | 输入 | 预期行为 |
|---|---------|------|---------|
| 1 | `test_estimate_tokens_empty_string` | `""` | 返回 0 |
| 2 | `test_estimate_tokens_english_text` | `"Hello world"` | 返回 2-5 之间 |
| 3 | `test_estimate_tokens_cjk_text` | `"你好世界"` | 返回 2-5 之间 |
| 4 | `test_estimate_tokens_mixed_cjk_english` | `"Hello 你好 world 世界"` | CJK 和英文混合，合理范围 |
| 5 | `test_estimate_tokens_numbers_and_symbols` | `"123 + 456 = 579"` | 数字和符号按英文字符计算 |
| 6 | `test_estimate_tokens_only_whitespace` | `"   \n\t"` | 空白字符有合理估算 |
| 7 | `test_estimate_tokens_long_english` | 1000 字符英文段落 | 约 250 个 token（4 chars/token） |
| 8 | `test_estimate_tokens_long_cjk` | 1000 字符中文段落 | 约 667 个 token（1.5 chars/token） |
| 9 | `test_estimate_tokens_emoji` | `"🎉🚀💡"` | emoji 按多字节字符处理 |
| 10 | `test_estimate_tokens_code_snippet` | Rust/Python 代码片段 | 合理估算 |

### 1.2 配额周期初始化 (`service/quota.rs::init_quota_period`)

| # | 测试名称 | 输入 cycle | 预期行为 |
|---|---------|-----------|---------|
| 1 | `test_init_quota_period_hourly` | `"hourly"` | period_start 为当前整点，period_end 为下一整点 |
| 2 | `test_init_quota_period_daily` | `"daily"` | period_start 为今天 00:00，period_end 为明天 00:00 |
| 3 | `test_init_quota_period_weekly` | `"weekly"` | period_start 为本周一 00:00，period_end 为下周一 00:00 |
| 4 | `test_init_quota_period_monthly` | `"monthly"` | period_start 为本月1日 00:00，period_end 为下月1日 00:00 |
| 5 | `test_init_quota_period_permanent` | `"permanent"` | 返回 `(None, None)` |
| 6 | `test_init_quota_period_unknown` | `"yearly"` | 返回 `(None, None)`（未知类型视为永久） |
| 7 | `test_init_quota_period_case_insensitive` | `"HOURLY"`, `"Daily"` | 大小写不敏感，正常处理 |

### 1.3 配额周期过期重置 (`state/app_state.rs::reset_quota_if_expired`)

| # | 测试名称 | 场景 | 预期行为 |
|---|---------|------|---------|
| 1 | `test_reset_hourly_not_expired` | period_end 在未来 | 返回 None，used 不变 |
| 2 | `test_reset_hourly_expired` | period_end 在过去 | used 重置为 0，返回新 period_end |
| 3 | `test_reset_daily_not_expired` | 当天内 | 返回 None，used 不变 |
| 4 | `test_reset_daily_expired` | 跨天 | used 重置为 0，返回新 period_end |
| 5 | `test_reset_weekly_not_expired` | 同周内 | 返回 None，used 不变 |
| 6 | `test_reset_weekly_expired` | 跨周 | used 重置为 0，返回新 period_end |
| 7 | `test_reset_monthly_not_expired` | 同月内 | 返回 None，used 不变 |
| 8 | `test_reset_monthly_expired` | 跨月 | used 重置为 0，返回新 period_end（30天） |
| 9 | `test_reset_permanent_never_expires` | cycle="permanent" | 永远返回 None，used 不变 |
| 10 | `test_reset_hourly_boundary` | period_end == now | 边界情况，应视为过期 |
| 11 | `test_reset_with_nonzero_used` | used=500, 过期 | used 重置为 0 |

### 1.4 流量方案时段验证 (`router/admin_routes.rs::validate_traffic_plan_slots`)

| # | 测试名称 | 输入 | 预期行为 |
|---|---------|------|---------|
| 1 | `test_validate_slots_valid_single` | `[{start:0, end:8, weight:70}]` | Ok |
| 2 | `test_validate_slots_valid_multiple` | `[{0,8,70}, {8,18,50}, {18,24,60}]` | Ok（覆盖全天） |
| 3 | `test_validate_slots_start_out_of_range_negative` | `[{start:-1, end:8, weight:70}]` | Err |
| 4 | `test_validate_slots_start_out_of_range_24` | `[{start:24, end:25, weight:70}]` | Err |
| 5 | `test_validate_slots_end_out_of_range_25` | `[{start:0, end:25, weight:70}]` | Err |
| 6 | `test_validate_slots_end_zero` | `[{start:0, end:0, weight:70}]` | Err |
| 7 | `test_validate_slots_start_equals_end` | `[{start:8, end:8, weight:70}]` | Err |
| 8 | `test_validate_slots_start_greater_than_end` | `[{start:18, end:8, weight:70}]` | Err |
| 9 | `test_validate_slots_weight_zero` | `[{start:0, end:8, weight:0}]` | Err |
| 10 | `test_validate_slots_weight_negative` | `[{start:0, end:8, weight:-10}]` | Err |
| 11 | `test_validate_slots_overlap` | `[{0,12,70}, {8,18,50}]` | Err（8-12 重叠） |
| 12 | `test_validate_slots_adjacent_no_overlap` | `[{0,8,70}, {8,16,50}]` | Ok（边界相接不重叠） |
| 13 | `test_validate_slots_empty_list` | `[]` | Ok（空列表允许） |
| 14 | `test_validate_slots_boundary_0_24` | `[{start:0, end:24, weight:100}]` | Ok（覆盖全天） |
| 15 | `test_validate_slots_three_way_overlap` | `[{0,10,70}, {8,14,50}, {12,20,30}]` | Err |

### 1.5 OpenAI Token 提取 (`service/openai.rs::extract_tokens`)

| # | 测试名称 | 输入 JSON | 预期行为 |
|---|---------|----------|---------|
| 1 | `test_extract_tokens_valid` | `{"usage":{"prompt_tokens":10,"completion_tokens":20,"total_tokens":30}}` | Some((10, 20, 30)) |
| 2 | `test_extract_tokens_missing_total` | `{"usage":{"prompt_tokens":10,"completion_tokens":20}}` | Some((10, 20, 30))（自动计算 total） |
| 3 | `test_extract_tokens_missing_usage` | `{}` | None |
| 4 | `test_extract_tokens_null_usage` | `{"usage":null}` | None |
| 5 | `test_extract_tokens_invalid_json` | `"not json"` | None |
| 6 | `test_extract_tokens_zero_values` | `{"usage":{"prompt_tokens":0,"completion_tokens":0,"total_tokens":0}}` | Some((0, 0, 0)) |
| 7 | `test_extract_tokens_large_values` | `{"usage":{"prompt_tokens":99999,"completion_tokens":88888,"total_tokens":188887}}` | Some((99999, 88888, 188887)) |
| 8 | `test_extract_tokens_extra_fields` | `{"usage":{"prompt_tokens":10,"completion_tokens":20,"total_tokens":30,"foo":"bar"}}` | Some((10, 20, 30)) |

### 1.6 Anthropic Token 提取 (`service/anthropic.rs::extract_tokens`)

| # | 测试名称 | 输入 JSON | 预期行为 |
|---|---------|----------|---------|
| 1 | `test_anthropic_extract_tokens_valid` | `{"usage":{"input_tokens":10,"output_tokens":20}}` | Some((10, 20, 30)) |
| 2 | `test_anthropic_extract_tokens_missing_output` | `{"usage":{"input_tokens":10}}` | Some((10, 0, 10)) |
| 3 | `test_anthropic_extract_tokens_missing_usage` | `{}` | None |
| 4 | `test_anthropic_extract_tokens_invalid_json` | `"not json"` | None |

### 1.7 Anthropic 流式检测 (`service/anthropic.rs::is_streaming_response`)

| # | 测试名称 | 输入 JSON | 预期行为 |
|---|---------|----------|---------|
| 1 | `test_is_streaming_content_block_start` | `{"type":"content_block_start"}` | true |
| 2 | `test_is_streaming_content_block_delta` | `{"type":"content_block_delta"}` | true |
| 3 | `test_is_streaming_message_start` | `{"type":"message_start"}` | true |
| 4 | `test_is_streaming_message_delta` | `{"type":"message_delta"}` | true |
| 5 | `test_is_streaming_message_stop` | `{"type":"message_stop"}` | true |
| 6 | `test_is_streaming_ping` | `{"type":"ping"}` | true |
| 7 | `test_is_streaming_error` | `{"type":"error"}` | true |
| 8 | `test_is_streaming_explicit_flag` | `{"stream":true}` | true |
| 9 | `test_is_not_streaming_full_message` | `{"id":"msg_123","type":"message"}` | false |
| 10 | `test_is_streaming_invalid_json` | `"not json"` | false |

### 1.8 OpenAI 流式解析 (`service/openai.rs::parse_stream_chunk`)

| # | 测试名称 | 输入 SSE 行 | 预期行为 |
|---|---------|------------|---------|
| 1 | `test_parse_stream_chunk_valid` | `data: {"choices":[{"delta":{"content":"Hi"}}]}` | Some(JSON value) |
| 2 | `test_parse_stream_chunk_done` | `data: [DONE]` | None（表示结束） |
| 3 | `test_parse_stream_chunk_empty` | `data: {}` | Some(empty object) |
| 4 | `test_parse_stream_chunk_invalid_data` | `data: not json` | None |
| 5 | `test_parse_stream_chunk_non_data_line` | `:comment` | None（忽略注释行） |
| 6 | `test_parse_stream_chunk_empty_line` | `""` | None |

### 1.9 Anthropic 流式解析 (`service/anthropic.rs::parse_stream_chunk`)

| # | 测试名称 | 输入 SSE 行 | 预期行为 |
|---|---------|------------|---------|
| 1 | `test_anthropic_parse_chunk_valid` | `data: {"type":"content_block_delta","delta":{"text":"Hi"}}` | Some(JSON value) |
| 2 | `test_anthropic_parse_chunk_done` | `data: [DONE]` | None |
| 3 | `test_anthropic_parse_chunk_invalid` | `data: not json` | None |

### 1.10 JWT 令牌签发与验证 (`utils/jwt.rs`)

| # | 测试名称 | 场景 | 预期行为 |
|---|---------|------|---------|
| 1 | `test_jwt_generate_and_verify_valid` | 生成 token 后立即验证 | Ok(Claims)，字段正确 |
| 2 | `test_jwt_verify_wrong_secret` | 用不同 secret 验证 | Err |
| 3 | `test_jwt_verify_expired` | 生成过期 token（exp 设为过去） | Err |
| 4 | `test_jwt_verify_tampered` | 修改 token 字符串后验证 | Err |
| 5 | `test_jwt_claims_correct` | 检查 user_id, username, role | 与输入一致 |
| 6 | `test_jwt_round_trip_multiple_users` | 不同用户生成/验证 | 各自 Claims 正确 |

---

## 二、业务逻辑测试（需要构造测试数据）

### 2.1 渠道 API Key 轮询选择 (`db/models.rs::get_channel_api_key`)

| # | 测试名称 | 场景 | 预期行为 |
|---|---------|------|---------|
| 1 | `test_get_channel_api_key_single_key` | api_keys = `["key1"]` | 始终返回 "key1" |
| 2 | `test_get_channel_api_key_round_robin` | api_keys = `["key1","key2","key3"]`, quota_used=0,1,2,3 | 依次返回 key1,key2,key3,key1 |
| 3 | `test_get_channel_api_key_empty_keys` | api_keys = `[]` | 返回错误 |
| 4 | `test_get_channel_api_key_invalid_json` | api_keys = `"not json"` | 返回错误 |
| 5 | `test_get_channel_api_key_null_keys` | api_keys = `null` | 返回错误 |
| 6 | `test_get_channel_api_key_large_quota_used` | quota_used=100, keys=3 | 返回 keys[100%3] = keys[1] |

### 2.2 代理请求头构建 (`service/proxy.rs::build_headers`)

| # | 测试名称 | 场景 | 预期行为 |
|---|---------|------|---------|
| 1 | `test_build_headers_openai_no_profile` | channel_type="openai", 无 app_profile | Authorization: Bearer {key}, Content-Type: application/json |
| 2 | `test_build_headers_anthropic_no_profile` | channel_type="anthropic", 无 app_profile | x-api-key: {key}, Content-Type: application/json, anthropic-version header |
| 3 | `test_build_headers_with_app_profile_ua` | app_profile 有 user_agent | 包含自定义 User-Agent |
| 4 | `test_build_headers_with_app_profile_extra_headers` | app_profile 有 extra_headers `{"X-Custom":"value"}` | 包含 X-Custom: value |
| 5 | `test_build_headers_with_channel_custom_headers` | channel 有 custom_headers `{"X-Channel":"ch"}` | 包含 X-Channel: ch |
| 6 | `test_build_headers_channel_overrides_profile` | profile 和 channel 都有 X-Same header | channel 的值覆盖 profile |
| 7 | `test_build_headers_invalid_extra_headers_json` | extra_headers = `"not json"` | 忽略 extra_headers，不报错 |
| 8 | `test_build_headers_invalid_custom_headers_json` | custom_headers = `"not json"` | 忽略 custom_headers，不报错 |
| 9 | `test_build_headers_multiple_api_keys` | 多 key 渠道 | 使用 select_api_key 轮询选择 |

### 2.3 API Key 轮询选择 (`service/proxy.rs::select_api_key`)

| # | 测试名称 | 场景 | 预期行为 |
|---|---------|------|---------|
| 1 | `test_select_api_key_single` | keys=["k1"], quota_used=0 | "k1" |
| 2 | `test_select_api_key_round_robin_sequence` | keys=["k1","k2"], quota_used=0,1,2,3 | k1,k2,k1,k2 |
| 3 | `test_select_api_key_empty` | keys=[] | 返回错误 |
| 4 | `test_select_api_key_large_index` | keys=["k1","k2"], quota_used=999 | keys[999%2] = "k2" |

### 2.4 用户配额检查 (`service/quota.rs::check_user_quota`)

> 需要 mock 数据库查询

| # | 测试名称 | 场景 | 预期行为 |
|---|---------|------|---------|
| 1 | `test_check_user_quota_no_quotas` | 用户无配额配置 | Ok（允许） |
| 2 | `test_check_user_quota_requests_under_limit` | limit=100, used=50 | Ok |
| 3 | `test_check_user_quota_requests_at_limit` | limit=100, used=100 | Err(Exceeded) |
| 4 | `test_check_user_quota_requests_over_limit` | limit=100, used=150 | Err(Exceeded) |
| 5 | `test_check_user_quota_tokens_under_limit` | limit=10000, used=5000, estimated=100 | Ok |
| 6 | `test_check_user_quota_tokens_at_limit` | limit=10000, used=9900, estimated=100 | Err(Exceeded) |
| 7 | `test_check_user_quota_token_estimate_overflow` | estimated > i32::MAX | 使用 i32::MAX |
| 8 | `test_check_user_quota_multiple_quotas_all_pass` | 两个配额都未超限 | Ok |
| 9 | `test_check_user_quota_multiple_quotas_one_fail` | 一个超限一个未超 | Err(Exceeded) |
| 10 | `test_check_user_quota_disabled_quota` | enabled=false | 跳过该配额检查 |

### 2.5 用户配额扣除 (`service/quota.rs::deduct_user_quota`)

| # | 测试名称 | 场景 | 预期行为 |
|---|---------|------|---------|
| 1 | `test_deduct_user_quota_requests` | quota_type="requests" | used +1 |
| 2 | `test_deduct_user_quota_tokens` | quota_type="tokens", tokens=500 | used +500 |
| 3 | `test_deduct_user_quota_no_quotas` | 无配额 | 无操作，Ok |
| 4 | `test_deduct_user_quota_disabled` | enabled=false | 跳过 |

### 2.6 渠道配额检查 (`service/quota.rs::check_channel_quota`)

| # | 测试名称 | 场景 | 预期行为 |
|---|---------|------|---------|
| 1 | `test_check_channel_quota_no_limit` | quota_limit=null | Ok（不限） |
| 2 | `test_check_channel_quota_under_limit` | limit=1000, used=500 | Ok |
| 3 | `test_check_channel_quota_at_limit` | limit=1000, used=1000 | Err(Exceeded) |
| 4 | `test_check_channel_quota_expired_reset` | 周期过期 | 先重置再检查 |
| 5 | `test_check_channel_quota_tokens_type` | quota_type="tokens" | 按 token 数检查 |

### 2.7 渠道配额扣除 (`service/quota.rs::deduct_channel_quota`)

| # | 测试名称 | 场景 | 预期行为 |
|---|---------|------|---------|
| 1 | `test_deduct_channel_quota_requests` | quota_type="requests" | quota_used +1 |
| 2 | `test_deduct_channel_quota_tokens` | quota_type="tokens", tokens=300 | quota_used +300 |
| 3 | `test_deduct_channel_quota_no_quota` | 无配额配置 | 无操作 |

### 2.8 渠道配额周期重置 (`service/quota.rs::reset_channel_quota_if_expired`)

| # | 测试名称 | 场景 | 预期行为 |
|---|---------|------|---------|
| 1 | `test_reset_channel_quota_hourly_not_expired` | period_end 在未来 | 不重置 |
| 2 | `test_reset_channel_quota_hourly_expired` | period_end 在过去 | 重置 used=0，更新周期 |
| 3 | `test_reset_channel_quota_daily` | 跨天 | 重置 |
| 4 | `test_reset_channel_quota_weekly` | 跨周 | 重置 |
| 5 | `test_reset_channel_quota_monthly` | 跨月 | 重置（30天周期） |
| 6 | `test_reset_channel_quota_permanent` | cycle="permanent" | 永不重置 |
| 7 | `test_reset_channel_quota_no_period_end` | period_end=null | 初始化周期 |

### 2.9 流量方案应用解析 (`db/models.rs::resolve_app_profile_for_channel`)

> 需要数据库，使用内存 SQLite + 测试数据

| # | 测试名称 | 场景 | 预期行为 |
|---|---------|------|---------|
| 1 | `test_resolve_channel_plan_no_plan` | 无渠道方案也无全局方案 | 返回渠道 app_profile_id |
| 2 | `test_resolve_channel_plan_channel_plan_match` | 有渠道方案，当前小时匹配 | 返回方案中匹配的 app_profile |
| 3 | `test_resolve_channel_plan_channel_plan_no_match` | 有渠道方案，当前小时不匹配 | 回退到全局方案 |
| 4 | `test_resolve_channel_plan_global_plan_match` | 无渠道方案，有全局方案且匹配 | 返回全局方案中匹配的 app_profile |
| 5 | `test_resolve_channel_plan_no_plan_no_legacy` | 无任何方案，渠道无 app_profile_id | 返回 None |
| 6 | `test_resolve_channel_plan_weighted_selection` | 方案有多个同小时槽，权重 70/30 | 多次调用，比例接近 70:30 |
| 7 | `test_resolve_channel_plan_empty_slots` | 方案存在但 slots 为空 | 回退到全局方案 |
| 8 | `test_resolve_channel_plan_boundary_hour_0` | 当前小时=0，槽 0-8 | 匹配 |
| 9 | `test_resolve_channel_plan_boundary_hour_23` | 当前小时=23，槽 18-24 | 匹配 |
| 10 | `test_resolve_channel_plan_hour_exactly_at_end` | 当前小时=8，槽 0-8 | 不匹配（end 是开区间） |

---

## 三、中间件测试（需要构造 HTTP 请求）

### 3.1 用户 API Key 认证 (`middleware/auth.rs`)

| # | 测试名称 | 场景 | 预期行为 |
|---|---------|------|---------|
| 1 | `test_auth_valid_api_key` | 有效 active 用户的 API Key | Ok，extensions 包含 user_id, role, api_key |
| 2 | `test_auth_missing_header` | 无 Authorization header | Err(MissingHeader) |
| 3 | `test_auth_invalid_key` | 不存在的 API Key | Err(InvalidKey) |
| 4 | `test_auth_disabled_user` | 用户 status="disabled" | Err(Disabled) |
| 5 | `test_auth_bearer_prefix` | `Bearer sk-xxx` 格式 | 正确提取 key |
| 6 | `test_auth_no_bearer_prefix` | 直接 `sk-xxx` | 也能识别 |
| 7 | `test_auth_db_error` | 数据库查询失败 | Err(Db) |
| 8 | `test_auth_empty_token` | Authorization: "Bearer " | Err(InvalidKey) |

### 3.2 管理员 JWT 认证 (`middleware/admin_auth.rs`)

| # | 测试名称 | 场景 | 预期行为 |
|---|---------|------|---------|
| 1 | `test_admin_auth_valid_admin_token` | 有效 admin 角色的 JWT | Ok |
| 2 | `test_admin_auth_valid_user_token` | 有效 user 角色的 JWT | Err（非 admin） |
| 3 | `test_admin_auth_missing_header` | 无 Authorization | Err |
| 4 | `test_admin_auth_invalid_token` | 伪造/篡改的 JWT | Err |
| 5 | `test_admin_auth_expired_token` | 过期的 JWT | Err |
| 6 | `test_admin_auth_disabled_user` | status="disabled" 的用户 | Err |
| 7 | `test_admin_auth_db_error` | 数据库查询失败 | Err |

### 3.3 用户 JWT 认证 (`middleware/user_auth.rs`)

| # | 测试名称 | 场景 | 预期行为 |
|---|---------|------|---------|
| 1 | `test_user_auth_valid_token` | 有效 JWT | Ok |
| 2 | `test_user_auth_invalid_token` | 无效 JWT | Err |
| 3 | `test_user_auth_disabled` | status="disabled" | Err |

### 3.4 全局速率限制 (`middleware/rate_limit.rs`)

| # | 测试名称 | 场景 | 预期行为 |
|---|---------|------|---------|
| 1 | `test_rate_limit_under_qps` | QPS 限制=10，发送 5 个请求 | 全部通过 |
| 2 | `test_rate_limit_exceed_qps` | QPS 限制=1，快速发送 3 个请求 | 第 2、3 个被拒绝 |
| 3 | `test_rate_limit_under_concurrency` | 并发限制=5，3 个并发 | 全部通过 |
| 4 | `test_rate_limit_exceed_concurrency` | 并发限制=1，2 个并发 | 第 2 个被拒绝 |
| 5 | `test_rate_limit_zero_means_unlimited` | QPS=0, concurrency=0 | 不限流 |
| 6 | `test_rate_limit_concurrency_decrement` | 请求完成后计数器递减 | 计数器正确回收 |

---

## 四、数据库 CRUD 测试（使用内存 SQLite）

### 4.1 用户 CRUD (`db/models.rs`)

| # | 测试名称 | 操作 | 验证点 |
|---|---------|------|--------|
| 1 | `test_create_user_success` | 创建用户 | 返回用户 ID，数据可查询 |
| 2 | `test_create_user_duplicate_username` | 创建同名用户 | 返回错误 |
| 3 | `test_get_user_by_api_key_valid` | 通过 API Key 查询 | 返回正确用户 |
| 4 | `test_get_user_by_api_key_invalid` | 不存在的 Key | 返回 None |
| 5 | `test_update_user_status` | 更新 status | 数据库值变更 |
| 6 | `test_update_user_enabled_models` | 更新 enabled_models | JSON 正确更新 |
| 7 | `test_delete_user` | 删除用户 | 查询返回 None |
| 8 | `test_list_users_pagination` | 分页查询 | 正确分页 |
| 9 | `test_list_users_filter_by_role` | 按 role 筛选 | 只返回匹配角色 |
| 10 | `test_list_users_filter_by_status` | 按 status 筛选 | 只返回匹配状态 |

### 4.2 渠道 CRUD (`db/models.rs`)

| # | 测试名称 | 操作 | 验证点 |
|---|---------|------|--------|
| 1 | `test_create_channel_success` | 创建渠道 | 返回 ID，api_keys 正确存储 |
| 2 | `test_create_channel_with_custom_headers` | 带 custom_headers | JSON 正确存储 |
| 3 | `test_get_active_channels` | 查询 active 渠道 | 只返回 status="active" |
| 4 | `test_update_channel_status` | 更新状态 | 数据库变更 |
| 5 | `test_delete_channel` | 删除渠道 | 查询返回 None |
| 6 | `test_list_channels_pagination` | 分页查询 | 正确分页 |

### 4.3 模型 CRUD (`db/models.rs`)

| # | 测试名称 | 操作 | 验证点 |
|---|---------|------|--------|
| 1 | `test_create_model_success` | 创建模型映射 | 返回 ID |
| 2 | `test_get_model_by_proxy_name` | 通过代理名查询 | 返回正确模型 |
| 3 | `test_get_models_by_channel` | 查询渠道下所有模型 | 返回列表 |
| 4 | `test_update_model_enabled` | 更新 enabled 状态 | 数据库变更 |
| 5 | `test_delete_model` | 删除模型 | 查询返回 None |
| 6 | `test_find_channels_for_model` | 查找支持某模型的渠道 | 返回匹配的 active 渠道 |

### 4.4 应用预设 CRUD (`db/models.rs`)

| # | 测试名称 | 操作 | 验证点 |
|---|---------|------|--------|
| 1 | `test_create_app_profile_success` | 创建应用预设 | 返回 ID |
| 2 | `test_create_app_profile_extra_headers` | 带 extra_headers | JSON 正确存储 |
| 3 | `test_update_system_profile_blocked` | 更新 is_system=true 的预设 | affected_rows=0 |
| 4 | `test_delete_system_profile_blocked` | 删除 is_system=true 的预设 | affected_rows=0 |
| 5 | `test_delete_user_profile_success` | 删除非系统预设 | 成功删除 |
| 6 | `test_get_app_profile_by_identifier` | 通过标识查询 | 返回正确预设 |

### 4.5 配额 CRUD (`db/models.rs`)

| # | 测试名称 | 操作 | 验证点 |
|---|---------|------|--------|
| 1 | `test_create_quota_success` | 创建配额 | 返回 ID，周期正确 |
| 2 | `test_create_quota_requests_type` | quota_type="requests" | 正确存储 |
| 3 | `test_create_quota_tokens_type` | quota_type="tokens" | 正确存储 |
| 4 | `test_get_active_quotas` | 查询活跃配额 | 只返回 enabled=true |
| 5 | `test_update_quota_limit` | 更新 total_limit | 数据库变更 |
| 6 | `test_increment_quota_used` | 增加 used 计数 | used 正确递增 |
| 7 | `test_delete_quota` | 删除配额 | 查询返回 None |

### 4.6 限流配置 CRUD (`db/models.rs`)

| # | 测试名称 | 操作 | 验证点 |
|---|---------|------|--------|
| 1 | `test_create_ratelimit_global` | 创建全局限流 | target_type="global" |
| 2 | `test_create_ratelimit_user` | 创建用户限流 | target_type="user", target_id 正确 |
| 3 | `test_create_ratelimit_channel` | 创建渠道限流 | target_type="channel", target_id 正确 |
| 4 | `test_update_ratelimit` | 更新 QPS/并发 | 数据库变更 |
| 5 | `test_delete_ratelimit` | 删除限流规则 | 查询返回 None |

### 4.7 请求日志 CRUD (`db/models.rs`)

| # | 测试名称 | 操作 | 验证点 |
|---|---------|------|--------|
| 1 | `test_insert_request_log` | 插入日志 | 返回 ID |
| 2 | `test_insert_request_log_with_bodies` | 带请求/响应体 | JSON 正确存储 |
| 3 | `test_list_logs_filter_by_model` | 按模型筛选 | 只返回匹配日志 |
| 4 | `test_list_logs_filter_by_status_code` | 按状态码筛选 | 只返回匹配日志 |
| 5 | `test_list_logs_filter_by_date_range` | 按日期范围筛选 | 只返回范围内日志 |
| 6 | `test_list_logs_filter_by_user` | 按用户筛选 | 只返回该用户日志 |
| 7 | `test_list_logs_pagination` | 分页查询 | 正确分页 |
| 8 | `test_cleanup_old_logs` | 清理 N 天前日志 | 旧日志被删除 |
| 9 | `test_get_request_log_by_id` | 通过 ID 查询 | 返回正确日志 |
| 10 | `test_delete_request_log` | 删除日志 | 查询返回 None |

### 4.8 流量方案 CRUD (`db/models.rs`)

| # | 测试名称 | 操作 | 验证点 |
|---|---------|------|--------|
| 1 | `test_create_global_traffic_plan` | channel_id=NULL | 创建全局方案 |
| 2 | `test_create_channel_traffic_plan` | channel_id=具体值 | 创建渠道方案 |
| 3 | `test_insert_traffic_plan_slot` | 插入时段 | 返回 ID |
| 4 | `test_get_traffic_plan_detail` | 查询方案详情 | 包含 slots 和 app_profile 信息 |
| 5 | `test_list_all_traffic_plans` | 列出所有方案 | 全局 + 渠道方案 |
| 6 | `test_delete_traffic_plan` | 删除方案 | 方案及 slots 被删除 |
| 7 | `test_delete_slots_by_plan_id` | 删除方案所有时段 | slots 被清空 |

### 4.9 用户 Key CRUD (`db/models.rs`)

| # | 测试名称 | 操作 | 验证点 |
|---|---------|------|--------|
| 1 | `test_create_user_key_success` | 创建用户 Key | 返回 ID，key_value 唯一 |
| 2 | `test_create_user_key_with_models` | 带 enabled_models | JSON 正确存储 |
| 3 | `test_get_user_key_by_value` | 通过 Key 值查询 | 返回正确 Key |
| 4 | `test_list_user_keys_by_user` | 查询用户所有 Key | 返回列表 |
| 5 | `test_update_user_key_status` | 更新状态 | 数据库变更 |
| 6 | `test_update_user_key_models` | 更新 enabled_models | JSON 正确更新 |
| 7 | `test_delete_user_key` | 删除 Key | 查询返回 None |

---

## 五、服务层测试

### 5.1 渠道健康检查 (`service/health.rs`)

> 需要 mock HTTP 客户端

| # | 测试名称 | 场景 | 预期行为 |
|---|---------|------|---------|
| 1 | `test_health_check_success` | /models 返回 200 | consecutive_failures 重置为 0 |
| 2 | `test_health_check_5xx_failure` | /models 返回 500 | consecutive_failures +1 |
| 3 | `test_health_check_connection_error` | 连接失败 | consecutive_failures +1 |
| 4 | `test_health_check_disable_at_3_failures` | 连续 3 次失败 | 渠道 status 设为 disabled |
| 5 | `test_health_check_recovery` | 失败后恢复成功 | consecutive_failures 重置，status 恢复 active |
| 6 | `test_health_check_disabled_channel_skipped` | 渠道已禁用 | 跳过检查 |
| 7 | `test_health_check_returns_disabled_and_recovered` | 有禁用和恢复的渠道 | 返回正确的 disabled_ids 和 recovered_ids |

### 5.2 OpenAI 请求适配 (`service/openai.rs::adapt_request`)

| # | 测试名称 | 场景 | 预期行为 |
|---|---------|------|---------|
| 1 | `test_adapt_request_no_model_map` | model_map 为空 | 请求不变 |
| 2 | `test_adapt_request_with_model_map` | model_map={"gpt-4": "gpt-4o"} | model 字段被替换 |
| 3 | `test_adapt_request_model_not_in_map` | 模型不在 map 中 | 保持原模型名 |
| 4 | `test_adapt_request_preserve_other_fields` | 请求有其他字段 | 其他字段不变 |

### 5.3 请求日志服务 (`service/log.rs`)

| # | 测试名称 | 场景 | 预期行为 |
|---|---------|------|---------|
| 1 | `test_log_request_success` | 正常记录 | 日志插入数据库 |
| 2 | `test_log_request_with_error` | 带错误信息 | error_message 正确存储 |
| 3 | `test_log_request_with_token_counts` | 带 token 计数 | token 字段正确存储 |
| 4 | `test_cleanup_old_logs_wrapper` | 调用清理 | 委托给 models 层 |

---

## 六、配置测试

### 6.1 配置加载 (`config.rs`)

| # | 测试名称 | 场景 | 预期行为 |
|---|---------|------|---------|
| 1 | `test_load_config_default` | 默认 config.yaml | 加载成功，默认值正确 |
| 2 | `test_load_config_env_override_port` | CORIDE_PORT=9000 | 端口被覆盖 |
| 3 | `test_load_config_env_override_db_path` | CORIDE_DB_PATH=/tmp/test.db | 数据库路径被覆盖 |
| 4 | `test_load_config_env_override_jwt_secret` | CORIDE_JWT_SECRET=custom-secret | JWT secret 被覆盖 |
| 5 | `test_load_config_legacy_lp_prefix` | LP_PORT=9000（旧前缀） | 兼容旧前缀 |
| 6 | `test_load_config_coride_takes_precedence` | CORIDE_PORT=9000, LP_PORT=8000 | CORIDE_ 优先 |
| 7 | `test_load_config_missing_yaml` | config.yaml 不存在 | 使用默认值或报错 |

---

## 七、集成测试场景

### 7.1 完整代理请求流程

| # | 测试名称 | 场景 | 预期行为 |
|---|---------|------|---------|
| 1 | `test_full_proxy_flow_openai_success` | 完整流程：认证→鉴权→配额→路由→代理→日志 | 200，日志记录 |
| 2 | `test_full_proxy_flow_streaming_success` | 流式请求 | SSE 流正确返回 |
| 3 | `test_full_proxy_flow_auth_failed` | 无效 API Key | 401 |
| 4 | `test_full_proxy_flow_model_not_allowed` | 模型不在 enabled_models | 403 |
| 5 | `test_full_proxy_flow_quota_exceeded` | 用户配额超限 | 402 |
| 6 | `test_full_proxy_flow_channel_quota_exceeded` | 渠道配额超限 | 402 |
| 7 | `test_full_proxy_flow_all_channels_fail` | 所有渠道 5xx | 502 |
| 8 | `test_full_proxy_flow_retry_on_5xx` | 第一个渠道 5xx，第二个成功 | 重试成功 |
| 9 | `test_full_proxy_flow_no_retry_on_4xx` | 渠道返回 400 | 不重试，直接返回 |
| 10 | `test_full_proxy_flow_anthropic_format` | Anthropic 格式请求 | 正确转发到 /v1/messages |

### 7.2 管理后台 API 测试

| # | 测试名称 | 场景 | 预期行为 |
|---|---------|------|---------|
| 1 | `test_admin_login_success` | 正确用户名密码 | 返回 JWT token |
| 2 | `test_admin_login_failure` | 错误密码 | 返回错误 |
| 3 | `test_admin_crud_user` | 创建→查询→更新→删除 | 完整 CRUD |
| 4 | `test_admin_crud_channel` | 渠道完整生命周期 | 完整 CRUD |
| 5 | `test_admin_crud_model` | 模型完整生命周期 | 完整 CRUD |
| 6 | `test_admin_crud_quota` | 配额完整生命周期 | 完整 CRUD |
| 7 | `test_admin_crud_ratelimit` | 限流规则完整生命周期 | 完整 CRUD |
| 8 | `test_admin_crud_app_profile` | 应用预设完整生命周期 | 完整 CRUD |
| 9 | `test_admin_traffic_plan_crud` | 流量方案完整生命周期 | 完整 CRUD |
| 10 | `test_admin_user_key_crud` | 用户 Key 完整生命周期 | 完整 CRUD |
| 11 | `test_admin_dashboard_stats` | 获取仪表盘统计 | 返回统计数据 |
| 12 | `test_admin_usage_stats` | 获取使用统计 | 返回统计数据 |
| 13 | `test_admin_export_logs_csv` | 导出 CSV | 返回 CSV 格式数据 |

---

## 八、边界和异常场景测试

### 8.1 输入验证

| # | 测试名称 | 场景 | 预期行为 |
|---|---------|------|---------|
| 1 | `test_proxy_request_body_too_large` | Body > 10MB | 400 |
| 2 | `test_proxy_request_invalid_utf8` | 非 UTF-8 字节 | 400 |
| 3 | `test_proxy_request_invalid_json` | 非 JSON 内容 | 400 |
| 4 | `test_proxy_request_missing_model` | 无 model 字段 | 400 |
| 5 | `test_admin_create_user_empty_username` | 空用户名 | 400 |
| 6 | `test_admin_create_user_empty_password` | 空密码 | 400 |
| 7 | `test_admin_create_channel_empty_name` | 空名称 | 400 |
| 8 | `test_admin_set_log_level_invalid_value` | level="verbose" | 400 |
| 9 | `test_admin_set_rate_limit_invalid_qps` | qps=0 | 400 |
| 10 | `test_admin_create_ratelimit_invalid_target` | target_type="foo" | 400 |

### 8.2 并发安全

| # | 测试名称 | 场景 | 预期行为 |
|---|---------|------|---------|
| 1 | `test_concurrent_quota_deduction` | 多线程同时扣减配额 | used 计数正确，无竞态 |
| 2 | `test_concurrent_rate_limit_check` | 多线程并发请求 | 限流正确，无超发 |
| 3 | `test_concurrent_api_key_rotation` | 多线程轮询 API Key | 分布均匀 |

---

## 九、测试执行清单

### 第一阶段：纯函数测试（预计 30 分钟）

- [ ] 1.1 Token 估算（补充 7 个用例）
- [ ] 1.2 配额周期初始化（7 个用例）
- [ ] 1.3 配额周期过期重置（11 个用例）
- [ ] 1.4 流量方案时段验证（15 个用例）
- [ ] 1.5 OpenAI Token 提取（8 个用例）
- [ ] 1.6 Anthropic Token 提取（4 个用例）
- [ ] 1.7 Anthropic 流式检测（10 个用例）
- [ ] 1.8 OpenAI 流式解析（6 个用例）
- [ ] 1.9 Anthropic 流式解析（3 个用例）
- [ ] 1.10 JWT 令牌（6 个用例）

### 第二阶段：业务逻辑测试（预计 45 分钟）

- [ ] 2.1 渠道 API Key 轮询（6 个用例）
- [ ] 2.2 代理请求头构建（9 个用例）
- [ ] 2.3 API Key 轮询选择（4 个用例）
- [ ] 2.4 用户配额检查（10 个用例）
- [ ] 2.5 用户配额扣除（4 个用例）
- [ ] 2.6 渠道配额检查（5 个用例）
- [ ] 2.7 渠道配额扣除（3 个用例）
- [ ] 2.8 渠道配额周期重置（7 个用例）
- [ ] 2.9 流量方案应用解析（10 个用例）

### 第三阶段：中间件测试（预计 30 分钟）

- [ ] 3.1 用户 API Key 认证（8 个用例）
- [ ] 3.2 管理员 JWT 认证（7 个用例）
- [ ] 3.3 用户 JWT 认证（3 个用例）
- [ ] 3.4 全局速率限制（6 个用例）

### 第四阶段：数据库 CRUD 测试（预计 60 分钟）

- [ ] 4.1 用户 CRUD（10 个用例）
- [ ] 4.2 渠道 CRUD（6 个用例）
- [ ] 4.3 模型 CRUD（6 个用例）
- [ ] 4.4 应用预设 CRUD（6 个用例）
- [ ] 4.5 配额 CRUD（7 个用例）
- [ ] 4.6 限流配置 CRUD（5 个用例）
- [ ] 4.7 请求日志 CRUD（10 个用例）
- [ ] 4.8 流量方案 CRUD（7 个用例）
- [ ] 4.9 用户 Key CRUD（7 个用例）

### 第五阶段：服务层测试（预计 30 分钟）

- [ ] 5.1 渠道健康检查（7 个用例）
- [ ] 5.2 OpenAI 请求适配（4 个用例）
- [ ] 5.3 请求日志服务（4 个用例）

### 第六阶段：配置和集成测试（预计 45 分钟）

- [ ] 6.1 配置加载（7 个用例）
- [ ] 7.1 完整代理请求流程（10 个用例）
- [ ] 7.2 管理后台 API 测试（13 个用例）

### 第七阶段：边界和异常场景（预计 20 分钟）

- [ ] 8.1 输入验证（10 个用例）
- [ ] 8.2 并发安全（3 个用例）

---

## 统计汇总

| 阶段 | 测试用例数 | 优先级 |
|------|-----------|--------|
| 一、纯函数测试 | 80 | P0 |
| 二、业务逻辑测试 | 64 | P0 |
| 三、中间件测试 | 24 | P1 |
| 四、数据库 CRUD 测试 | 64 | P1 |
| 五、服务层测试 | 15 | P1 |
| 六、配置和集成测试 | 30 | P2 |
| 七、边界和异常场景 | 13 | P2 |
| **总计** | **290** | |

---

## 实施建议

1. **优先实现第一阶段纯函数测试** —— 无外部依赖，最容易编写和通过
2. **使用 `sqlx::sqlite::SqlitePoolOptions::new().connect(":memory:").await` 进行数据库测试**
3. **每个测试文件顶部添加测试辅助函数**：创建测试用户、测试渠道等
4. **使用 `#[tokio::test]` 标记异步测试**
5. **测试数据使用有意义的命名**：`test_user_active`, `test_channel_openai` 等
6. **测试完成后清理**：使用事务回滚或删除测试数据
7. **CI 集成**：确保 `cargo test` 能通过所有测试
