import { request } from '../request';

/** Get user list with pagination */
export function fetchUserList(page = 1, pageSize = 20) {
  return request<Api.User.User[]>({
    url: '/admin/users',
    method: 'get',
    params: { page, page_size: pageSize }
  });
}

/** Create a new user */
export function fetchCreateUser(data: Api.User.CreateUserParams) {
  return request<{ id: number; api_key: string }>({
    url: '/admin/users',
    method: 'post',
    data
  });
}

/** Reset user API key */
export function fetchResetUserKey(id: number) {
  return request<{ api_key: string }>({
    url: `/admin/users/${id}/reset-key`,
    method: 'post'
  });
}

/** Delete user */
export function fetchDeleteUser(id: number) {
  return request<{ deleted: boolean }>({
    url: `/admin/users/${id}`,
    method: 'delete'
  });
}

/** Update user */
export function fetchUpdateUser(id: number, data: Partial<Api.User.CreateUserParams>) {
  return request<{ updated: boolean }>({
    url: `/admin/users/${id}`,
    method: 'put',
    data
  });
}
