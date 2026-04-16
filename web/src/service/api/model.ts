import { request } from '../request';

/** Get model list with pagination */
export function fetchModelList(page = 1, pageSize = 20) {
  return request<Api.Model.Model[]>({
    url: '/admin/models',
    method: 'get',
    params: { page, page_size: pageSize }
  });
}

/** Create a new model */
export function fetchCreateModel(data: Api.Model.CreateModelParams) {
  return request<{ id: number }>({
    url: '/admin/models',
    method: 'post',
    data
  });
}

/** Delete model */
export function fetchDeleteModel(id: number) {
  return request<{ deleted: boolean }>({
    url: `/admin/models/${id}`,
    method: 'delete'
  });
}

/** Update model */
export function fetchUpdateModel(id: number, data: Partial<Api.Model.CreateModelParams>) {
  return request<{ updated: boolean }>({
    url: `/admin/models/${id}`,
    method: 'put',
    data
  });
}
