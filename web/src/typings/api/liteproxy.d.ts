/**
 * Namespace Api
 *
 * CoRide-API backend API types
 */
declare namespace Api {
  /** Auth types */
  namespace Auth {
    interface LoginToken {
      token: string;
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
    }

    interface CreateUserParams {
      username: string;
      password: string;
      role?: string;
      note?: string;
    }
  }

  /** Channel types */
  namespace Channel {
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
      quota_type?: string;
      quota_limit?: number;
      quota_used: number;
      quota_cycle?: string;
      quota_period_start?: string;
      quota_period_end?: string;
      app_profile_id?: number;
      created_at: string;
      updated_at: string;
      stats?: {
        total_requests: number;
        total_tokens: number;
        today_requests: number;
        today_tokens: number;
      };
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
      quota_type?: string;
      quota_limit?: number;
      quota_cycle?: string;
      app_profile_id?: number;
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
      period_start?: string;
      period_end?: string;
      enabled: boolean;
      created_at: string;
    }

    interface CreateQuotaParams {
      user_id: number;
      quota_type: string;
      total_limit: number;
      cycle: string;
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
      channel_id?: number;
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
  }

  /** Dashboard types */
  namespace Stats {
    interface DashboardStats {
      total_requests: number;
      today_requests: number;
      active_users: number;
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
}
