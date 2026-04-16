import { request } from '../request';

/** Get request log list with pagination and server-side filtering */
export function fetchLogList(page = 1, pageSize = 20, params?: { model?: string; status_code?: string }) {
  return request<Api.Log.RequestLog[]>({
    url: '/admin/logs',
    method: 'get',
    params: { page, page_size: pageSize, ...params }
  });
}

/** Get latest request logs (for dashboard real-time view) */
export function fetchRecentLogs(limit = 10) {
  return request<Api.Log.RequestLog[]>({
    url: '/admin/logs',
    method: 'get',
    params: { page: 1, page_size: limit }
  });
}

/** Export logs as CSV - uses native fetch to avoid responseType issues */
export function exportLogsCsv() {
  const base = import.meta.env.VITE_SERVICE_BASE_URL || '';
  const url = `${base}/admin/logs/export`;
  return fetch(url, { method: 'GET' });
}
