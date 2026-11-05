import { request } from '../request';

/** Get dashboard statistics (admin scope) */
export function fetchDashboardStats() {
  return request<Api.Stats.DashboardStats>({
    url: '/admin/stats/dashboard',
    method: 'get'
  });
}

/** Get dashboard statistics for the signed-in user */
export function fetchUserDashboardStats() {
  return request<Api.Stats.DashboardStats>({
    url: '/user/stats/dashboard',
    method: 'get'
  });
}

/** Get usage statistics (trend, channel usage, top users) with optional filters */
export function fetchUsageStats(params?: {
  user_api_key?: string;
  user_id?: number;
  channel_id?: number;
  model?: string;
  days?: number;
}) {
  return request<Api.Stats.UsageStats>({
    url: '/admin/stats/usage',
    method: 'get',
    params
  });
}

/** Get usage statistics for the signed-in user */
export function fetchUserUsageStats(params?: { days?: number; channel?: number; model?: string }) {
  return request<Api.Stats.UsageStats>({
    url: '/user/stats/usage',
    method: 'get',
    params
  });
}

/** Get quota warnings (users with usage > 80%) */
export function fetchQuotaWarnings() {
  return request<Api.Stats.QuotaWarningResult>({
    url: '/admin/quotas/warnings',
    method: 'get'
  });
}

/** Get the effective system configuration */
export function fetchSystemConfig() {
  return request<Api.System.Config>({
    url: '/admin/system/config',
    method: 'get'
  });
}

/** Update the log level / retention period at runtime */
export function fetchSetLogLevel(data: { level: string; retention_days?: number }) {
  return request<{ message: string; level: string; retention_days?: number }>({
    url: '/admin/system/config/log-level',
    method: 'put',
    data
  });
}

/** Update the global rate limit (applied immediately) */
export function fetchSetGlobalRateLimit(data: { qps: number; concurrency: number; action: string }) {
  return request<{ message: string; qps: number; concurrency: number; action: string }>({
    url: '/admin/system/config/rate-limit',
    method: 'put',
    data
  });
}
