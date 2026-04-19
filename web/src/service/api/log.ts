import { request } from '../request';

/** Get request log list with pagination and server-side filtering */
export function fetchLogList(page = 1, pageSize = 20, params?: { model?: string; status_code?: string; start_time?: string; end_time?: string }) {
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
  const token = localStorage.getItem('CORIDE_token');
  // Remove quotes from the stored token
  const cleanToken = token ? token.replace(/^"|"$/g, '') : '';
  return fetch(url, {
    method: 'GET',
    headers: {
      Authorization: cleanToken ? `Bearer ${cleanToken}` : ''
    }
  });
}
