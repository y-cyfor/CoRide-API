import { request } from '../request';

/** Get all rate limit configs */
export function fetchRateLimitList(page = 1, pageSize = 100) {
  return request<Api.Common.PageResult<Api.RateLimit.RateLimit>>({
    url: '/admin/ratelimits',
    method: 'get',
    params: { page, page_size: pageSize }
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
