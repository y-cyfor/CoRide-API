import { request } from '../request';

/** Get channel list with pagination */
export function fetchChannelList(page = 1, pageSize = 20) {
  return request<Api.Channel.Channel[]>({
    url: '/admin/channels',
    method: 'get',
    params: { page, page_size: pageSize }
  });
}

/** Create a new channel */
export function fetchCreateChannel(data: Api.Channel.CreateChannelParams) {
  return request<{ id: number }>({
    url: '/admin/channels',
    method: 'post',
    data
  });
}

/** Delete channel */
export function fetchDeleteChannel(id: number) {
  return request<{ deleted: boolean }>({
    url: `/admin/channels/${id}`,
    method: 'delete'
  });
}

/** Update channel */
export function fetchUpdateChannel(id: number, data: Partial<Api.Channel.CreateChannelParams>) {
  return request<{ updated: boolean }>({
    url: `/admin/channels/${id}`,
    method: 'put',
    data
  });
}

/** Test channel connectivity */
export function fetchTestChannel(id: number) {
  return request<{ status: string }>({
    url: `/admin/channels/${id}/test`,
    method: 'post'
  });
}
