import { request } from '../request';

/**
 * Login with username and password
 */
export function fetchLogin(username: string, password: string) {
  return request<Api.Auth.LoginToken>({
    url: '/admin/auth/login',
    method: 'post',
    data: {
      username,
      password
    }
  });
}

/** Get current user info */
export function fetchGetUserInfo() {
  return request<Api.Auth.UserInfo>({ url: '/admin/auth/me' });
}

/**
 * Refresh token
 */
export function fetchRefreshToken(refreshToken: string) {
  return request<Api.Auth.LoginToken>({
    url: '/admin/auth/refresh',
    method: 'post',
    data: {
      refresh_token: refreshToken
    }
  });
}

/**
 * return custom backend error
 */
export function fetchCustomBackendError(code: string, msg: string) {
  return request({ url: '/auth/error', params: { code, msg } });
}
