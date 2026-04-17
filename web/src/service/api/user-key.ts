import { request } from '../request';

/** Get current user's API keys */
export function fetchUserKeyList() {
  return request<Api.UserKey.UserKey[]>({
    url: '/user/keys',
    method: 'get'
  });
}

/** Create new API key */
export function fetchCreateUserKey(data: { name?: string; enabled_models?: string }) {
  return request<Api.UserKey.CreateResult>({
    url: '/user/keys',
    method: 'post',
    data
  });
}

/** Update API key */
export function fetchUpdateUserKey(id: number, data: { name?: string; enabled_models?: string; status?: string }) {
  return request<{ updated: boolean }>({
    url: `/user/keys/${id}`,
    method: 'put',
    data
  });
}

/** Delete API key */
export function fetchDeleteUserKey(id: number) {
  return request<{ deleted: boolean }>({
    url: `/user/keys/${id}`,
    method: 'delete'
  });
}

/** Get all user keys (admin only) */
export function fetchAllUserKeys() {
  return request<Api.UserKey.UserKeyWithUsername[]>({
    url: '/admin/keys',
    method: 'get'
  });
}

/** Get user's keys by user ID (admin only) */
export function fetchUserKeysByUserId(userId: number) {
  return request<Api.UserKey.UserKey[]>({
    url: `/admin/keys/${userId}`,
    method: 'get'
  });
}
