import { request } from '../request';

/** Get dashboard statistics */
export function fetchDashboardStats() {
  return request<Api.Stats.DashboardStats>({
    url: '/admin/stats/dashboard',
    method: 'get'
  });
}

/** Get usage statistics (trend, channel usage, top users) with optional filters */
export function fetchUsageStats(params?: {
  user_api_key?: string;
  channel_id?: number;
  model?: string;
  days?: number;
}) {
  return request<{
    daily_trend: Array<{ day: string; count: number }>;
    channel_usage: Array<{ name: string; count: number }>;
    top_users: Array<{ api_key: string; count: number }>;
    total_tokens: number;
    token_daily: Array<{ day: string; prompt_tokens: number; completion_tokens: number }>;
  }>({
    url: '/admin/stats/usage',
    method: 'get',
    params
  });
}

/** Get quota warnings (users with usage > 80%) */
export function fetchQuotaWarnings() {
  return request<{
    warnings: Array<{
      user_id: number;
      username: string;
      quota_type: string;
      total_limit: number;
      used: number;
      percent: number;
      severity: 'info' | 'warning' | 'critical';
      message: string;
    }>;
    total_warnings: number;
  }>({
    url: '/admin/quotas/warnings',
    method: 'get'
  });
}
