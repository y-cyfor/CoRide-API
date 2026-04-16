import { request } from '../request';

/** Get quota list with pagination */
export function fetchQuotaList(page = 1, pageSize = 20, userId?: number) {
  return request<Api.Quota.Quota[]>({
    url: '/admin/quotas',
    method: 'get',
    params: { page, page_size: pageSize, user_id: userId }
  });
}

/** Create a new quota */
export function fetchCreateQuota(data: Api.Quota.CreateQuotaParams) {
  return request<{ id: number }>({
    url: '/admin/quotas',
    method: 'post',
    data
  });
}

/** Delete quota */
export function fetchDeleteQuota(id: number) {
  return request<{ deleted: boolean }>({
    url: `/admin/quotas/${id}`,
    method: 'delete'
  });
}

/** Update quota */
export function fetchUpdateQuota(id: number, data: { total_limit?: number }) {
  return request<{ updated: boolean }>({
    url: `/admin/quotas/${id}`,
    method: 'put',
    data
  });
}
