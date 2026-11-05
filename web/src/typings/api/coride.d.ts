/**
 * Namespace Api
 *
 * CoRide-API backend API types
 */
declare namespace Api {
  /** Shared shapes */
  namespace Common {
    /** Paginated list payload returned by the admin endpoints. */
    interface PageResult<T> {
      items: T[];
      total: number;
    }

    /** Select option used across the management pages. */
    interface Option<T = number> {
      label: string;
      value: T;
    }
  }

  /** Auth types */
  namespace Auth {
    interface LoginToken {
      token: string;
      refreshToken?: string;
      user: {
        id: number;
        username: string;
        role: string;
      };
    }

    interface UserInfo {
      id: number;
      username: string;
      role: string;
    }
  }

  /** User types */
  namespace User {
    interface User {
      id: number;
      username: string;
      role: string;
      api_key: string;
      status: string;
      enabled_models?: string;
      note?: string;
      created_at: string;
      updated_at: string;
      /** Aggregated request quota (only set on list responses). */
      quota_total?: number;
      quota_used?: number;
      /** Aggregated token quota (only set on list responses). */
      token_quota_total?: number;
      token_quota_used?: number;
    }

    interface CreateUserParams {
      username: string;
      password: string;
      role?: string;
      note?: string;
      enabled_models?: string;
    }

    interface UpdateUserParams {
      username?: string;
      role?: string;
      status?: string;
      enabled_models?: string;
      note?: string;
    }

    interface CreateUserResult {
      id: number;
      api_key: string;
    }
  }

  /** Channel types */
  namespace Channel {
    interface ChannelStats {
      total_requests: number;
      total_tokens: number;
      today_requests: number;
      today_tokens: number;
    }

    interface Channel {
      id: number;
      name: string;
      type: string;
      base_url: string;
      api_keys: string;
      custom_headers?: string;
      status: string;
      weight: number;
      timeout: number;
      retry_count: number;
      quota_type?: string | null;
      quota_limit?: number | null;
      quota_used: number;
      quota_cycle?: string | null;
      quota_period_start?: string | null;
      quota_period_end?: string | null;
      app_profile_id?: number | null;
      proxy_url?: string | null;
      /** Set when the health checker disabled the channel automatically. */
      auto_disabled?: boolean;
      /** Consecutive failed health probes. */
      consecutive_failures?: number;
      last_checked?: string | null;
      last_success_at?: string | null;
      /** False when the stored API keys cannot be decrypted with the current secret. */
      api_keys_decryptable?: boolean;
      created_at: string;
      updated_at: string;
      stats?: ChannelStats;
    }

    interface CreateChannelParams {
      name: string;
      type: string;
      base_url: string;
      api_keys: string;
      custom_headers?: string;
      weight?: number;
      timeout?: number;
      retry_count?: number;
      quota_type?: string | null;
      quota_limit?: number | null;
      quota_cycle?: string | null;
      app_profile_id?: number | null;
      proxy_url?: string | null;
    }

    interface TestChannelResult {
      status: 'ok' | 'warning';
      http_status: number;
      message: string;
      latency_ms: number;
      response_body: string;
    }

    interface ChannelStatsDetail extends ChannelStats {
      quota_type?: string | null;
      quota_limit?: number | null;
      quota_used: number;
      quota_cycle?: string | null;
      quota_percent?: number | null;
    }
  }

  /** Model types */
  namespace Model {
    interface Model {
      id: number;
      channel_id: number;
      source_name: string;
      proxy_name: string;
      enabled: boolean;
      is_default: boolean;
      created_at: string;
    }

    interface CreateModelParams {
      channel_id: number;
      source_name: string;
      proxy_name: string;
    }
  }

  /** Quota types */
  namespace Quota {
    interface Quota {
      id: number;
      user_id: number;
      quota_type: string;
      total_limit: number;
      used: number;
      cycle: string;
      channel_id?: number | null;
      period_start?: string | null;
      period_end?: string | null;
      enabled: boolean;
      created_at: string;
    }

    interface CreateQuotaParams {
      user_id: number;
      quota_type: string;
      total_limit: number;
      cycle: string;
      channel_id?: number | null;
    }

    interface UpdateQuotaParams {
      quota_type?: string;
      total_limit?: number;
      cycle?: string;
      enabled?: boolean;
    }

    interface QuotaWarning {
      user_id: number;
      username: string;
      quota_type: string;
      total_limit: number;
      used: number;
      percent: number;
      severity: 'info' | 'warning' | 'critical';
      message: string;
    }
  }

  /** RateLimit types */
  namespace RateLimit {
    interface RateLimit {
      id: number;
      target_type: string;
      target_id: number | null;
      qps: number;
      concurrency: number;
      action: string;
      created_at: string;
    }

    interface CreateRateLimitParams {
      target_type: string;
      target_id: number | null;
      qps: number;
      concurrency: number;
      action: string;
    }
  }

  /** AppProfile types */
  namespace AppProfile {
    interface AppProfile {
      id: number;
      name: string;
      identifier: string;
      user_agent: string;
      extra_headers?: string;
      description?: string;
      enabled: boolean;
      is_system: boolean;
      created_at: string;
    }

    interface CreateAppProfileParams {
      name: string;
      identifier: string;
      user_agent: string;
      extra_headers?: string;
      description?: string;
    }
  }

  /** TrafficPlan types */
  namespace TrafficPlan {
    interface Slot {
      id: number;
      start_hour: number;
      end_hour: number;
      app_profile_id: number;
      app_profile_name: string;
      app_profile_identifier: string;
      weight: number;
    }

    interface PlanDetail {
      id: number;
      channel_id: number | null;
      slots: Slot[];
      created_at: string;
    }

    interface SlotRequest {
      start_hour: number;
      end_hour: number;
      app_profile_id: number;
      weight: number;
    }

    interface UpsertPlanParams {
      slots: SlotRequest[];
    }
  }

  /** RequestLog types */
  namespace Log {
    interface RequestLog {
      id: number;
      user_api_key: string;
      user_id?: number | null;
      key_id?: number | null;
      channel_id?: number | null;
      model: string;
      endpoint: string;
      status_code: number;
      prompt_tokens: number;
      completion_tokens: number;
      total_tokens: number;
      elapsed_ms: number;
      request_body?: string;
      response_body?: string;
      error_message?: string;
      created_at: string;
    }

    interface LogFilter {
      model?: string;
      status_code?: string;
      start_time?: string;
      end_time?: string;
      user_id?: number;
      api_key?: string;
    }
  }

  /** Statistics types */
  namespace Stats {
    interface DashboardStats {
      total_requests: number;
      today_requests: number;
      active_users: number;
      success_count?: number;
      failure_count?: number;
      p95_latency_ms?: number;
      error_rate?: string;
      total_tokens?: number;
    }

    interface UsageStats {
      daily_trend: Array<{ day: string; count: number }>;
      channel_usage: Array<{ name: string; count: number }>;
      top_users: Array<{ user_id?: number | null; username?: string; api_key: string; count: number }>;
      total_tokens: number;
      token_daily: Array<{ day: string; prompt_tokens: number; completion_tokens: number }>;
      model_usage: Array<{ name: string; count: number }>;
    }

    interface QuotaWarningResult {
      warnings: Quota.QuotaWarning[];
      total_warnings: number;
    }
  }

  /** UserKey types */
  namespace UserKey {
    interface UserKey {
      id: number;
      user_id: number;
      key_value: string;
      name?: string;
      enabled_models?: string;
      status: string;
      created_at: string;
    }

    interface UserKeyWithUsername {
      id: number;
      user_id: number;
      username: string;
      key_value: string;
      name?: string;
      enabled_models?: string;
      status: string;
      created_at: string;
    }

    interface CreateResult {
      id: number;
      key_value: string;
      name?: string;
      enabled_models?: string;
      status: string;
    }
  }

  /** IP Access types */
  namespace IpAccess {
    interface BlacklistEntry {
      id: number;
      ip_address: string;
      created_at: string;
    }

    interface WhitelistEntry {
      id: number;
      user_id: number;
      ip_address: string;
      created_at: string;
    }

    interface AddBlacklistParams {
      ip_address: string;
    }

    interface AddWhitelistParams {
      user_id: number;
      ip_address: string;
    }
  }

  /** System configuration */
  namespace System {
    interface Config {
      server: { host: string; port: number; cors_allowed_origins?: string[] | null };
      database: { path: string; pool_size: number };
      log: { level: string; retention_days: number; max_records: number };
      proxy: {
        timeout: number;
        max_retries: number;
        log_request_body: boolean;
        log_response_body: boolean;
        trusted_proxies: string[];
      };
      health_check: {
        enabled: boolean;
        interval_secs: number;
        failure_threshold: number;
        timeout_secs: number;
      };
      security: {
        encryption_key_count: number;
        encryption_secret_source: string;
      };
      global_rate_limit: { qps: number; concurrency: number; action: string };
    }
  }
}
