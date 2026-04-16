import { request } from '../request';

/** Get app profile list with pagination */
export function fetchAppProfileList(page = 1, pageSize = 20) {
  return request<Api.AppProfile.AppProfile[]>({
    url: '/admin/app-profiles',
    method: 'get',
    params: { page, page_size: pageSize }
  });
}

/** Create a new app profile */
export function fetchCreateAppProfile(data: Api.AppProfile.CreateAppProfileParams) {
  return request<{ id: number }>({
    url: '/admin/app-profiles',
    method: 'post',
    data
  });
}

/** Delete app profile */
export function fetchDeleteAppProfile(id: number) {
  return request<{ deleted: boolean }>({
    url: `/admin/app-profiles/${id}`,
    method: 'delete'
  });
}

/** Update app profile */
export function fetchUpdateAppProfile(id: number, data: Partial<Api.AppProfile.CreateAppProfileParams>) {
  return request<{ updated: boolean }>({
    url: `/admin/app-profiles/${id}`,
    method: 'put',
    data
  });
}

/** Get global traffic plan */
export function fetchGlobalTrafficPlan() {
  return request<Api.TrafficPlan.PlanDetail | null>({
    url: '/admin/traffic-plan/global',
    method: 'get'
  });
}

/** Create or update global traffic plan */
export function fetchUpsertGlobalTrafficPlan(data: Api.TrafficPlan.UpsertPlanParams) {
  return request<Api.TrafficPlan.PlanDetail>({
    url: '/admin/traffic-plan/global',
    method: 'put',
    data
  });
}

/** Delete global traffic plan */
export function fetchDeleteGlobalTrafficPlan() {
  return request<{ deleted: boolean }>({
    url: '/admin/traffic-plan/global',
    method: 'delete'
  });
}

/** List all per-channel traffic plans */
export function fetchChannelTrafficPlans() {
  return request<Api.TrafficPlan.PlanDetail[]>({
    url: '/admin/traffic-plan/channels',
    method: 'get'
  });
}

/** Get a specific channel's traffic plan */
export function fetchChannelTrafficPlan(channelId: number) {
  return request<Api.TrafficPlan.PlanDetail | null>({
    url: `/admin/traffic-plan/channel/${channelId}`,
    method: 'get'
  });
}

/** Create or update a channel's traffic plan */
export function fetchUpsertChannelTrafficPlan(channelId: number, data: Api.TrafficPlan.UpsertPlanParams) {
  return request<Api.TrafficPlan.PlanDetail>({
    url: `/admin/traffic-plan/channel/${channelId}`,
    method: 'put',
    data
  });
}

/** Delete a channel's traffic plan */
export function fetchDeleteChannelTrafficPlan(channelId: number) {
  return request<{ deleted: boolean }>({
    url: `/admin/traffic-plan/channel/${channelId}`,
    method: 'delete'
  });
}
