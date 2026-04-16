import { request } from '../request';

/** Get all rate limit configs */
export function fetchRateLimitList() {
  return request<Api.RateLimit.RateLimit[]>({
    url: '/admin/ratelimits',
    method: 'get'
  });
}

/** Create a new rate limit */
export function fetchCreateRateLimit(data: Api.RateLimit.CreateRateLimitParams) {
  return request<{ id: number }>({
    url: '/admin/ratelimits',
    method: 'post',
    data
  });
}

/** Delete rate limit */
export function fetchDeleteRateLimit(id: number) {
  return request<{ deleted: boolean }>({
    url: `/admin/ratelimits/${id}`,
    method: 'delete'
  });
}

/** Update rate limit */
export function fetchUpdateRateLimit(id: number, data: { qps?: number; concurrency?: number; action?: string }) {
  return request<{ updated: boolean }>({
    url: `/admin/ratelimits/${id}`,
    method: 'put',
    data
  });
}
