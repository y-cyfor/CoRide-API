import { request } from '../request';
import { localStg } from '@/utils/storage';

/** Get request log list with pagination and server-side filtering (admin) */
export function fetchLogList(page = 1, pageSize = 20, params?: Api.Log.LogFilter) {
  return request<Api.Common.PageResult<Api.Log.RequestLog>>({
    url: '/admin/logs',
    method: 'get',
    params: { page, page_size: pageSize, ...params }
  });
}

/** Get latest request logs (for the dashboard real-time view) */
export function fetchRecentLogs(limit = 10) {
  return request<Api.Common.PageResult<Api.Log.RequestLog>>({
    url: '/admin/logs',
    method: 'get',
    params: { page: 1, page_size: limit }
  });
}

/** Get the current user's own request logs */
export function fetchUserLogList(page = 1, pageSize = 20, params?: Api.Log.LogFilter) {
  return request<Api.Common.PageResult<Api.Log.RequestLog>>({
    url: '/user/logs',
    method: 'get',
    params: { page, page_size: pageSize, ...params }
  });
}

/** Export logs as CSV - uses native fetch to avoid responseType issues */
export function exportLogsCsv() {
  const base = import.meta.env.VITE_SERVICE_BASE_URL || '';
  const url = `${base}/admin/logs/export`;
  const token = (localStg.get('token') || '').replace(/"/g, '');
  return fetch(url, {
    method: 'GET',
    headers: {
      Authorization: token ? `Bearer ${token}` : ''
    }
  });
}
