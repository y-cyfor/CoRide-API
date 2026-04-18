import { request } from '../request';

// Blacklist
export function fetchBlacklistList() {
  return request<Api.IpAccess.BlacklistEntry[]>({ url: '/admin/ip/blacklist', method: 'get' });
}
export function fetchAddBlacklist(data: Api.IpAccess.AddBlacklistParams) {
  return request<{ id: number }>({ url: '/admin/ip/blacklist', method: 'post', data });
}
export function fetchDeleteBlacklist(id: number) {
  return request<{ deleted: boolean }>({ url: `/admin/ip/blacklist/${id}`, method: 'delete' });
}

// Whitelist
export function fetchUserWhitelist(userId: number) {
  return request<Api.IpAccess.WhitelistEntry[]>({ url: `/admin/ip/whitelist/user/${userId}`, method: 'get' });
}
export function fetchAddWhitelist(data: Api.IpAccess.AddWhitelistParams) {
  return request<{ id: number }>({ url: '/admin/ip/whitelist', method: 'post', data });
}
export function fetchDeleteWhitelist(id: number) {
  return request<{ deleted: boolean }>({ url: `/admin/ip/whitelist/entry/${id}`, method: 'delete' });
}
