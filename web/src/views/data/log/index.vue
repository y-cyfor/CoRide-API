<script setup lang="ts">
import { ref, h, onMounted } from 'vue';
import { NSpace, NTag, NCard, NDataTable, NButton, NDrawer, NDrawerContent, NDescriptions, NDescriptionsItem, NCode, useMessage, NSelect, NInput, NGrid, NGi, NDatePicker } from 'naive-ui';
import { fetchLogList, exportLogsCsv, fetchModelList } from '@/service/api';
import type { DataTableColumns } from 'naive-ui';

const loading = ref(false);
const logs = ref<Api.Log.RequestLog[]>([]);
const showDrawer = ref(false);
const selectedLog = ref<Api.Log.RequestLog | null>(null);
const message = useMessage();

// Filter state
const filterModel = ref<string>('');
const filterStatus = ref<string>('');
const filterDateRange = ref<[number, number] | null>(null);
const modelOptions = ref<{ label: string; value: string }[]>([]);

const statusOptions = [
  { label: '全部', value: '' },
  { label: '2xx 成功', value: '2' },
  { label: '3xx 重定向', value: '3' },
  { label: '4xx 客户端错误', value: '4' },
  { label: '5xx 服务器错误', value: '5' }
];

const pagination = ref({
  page: 1,
  pageSize: 20,
  itemCount: 0
});

const columns: DataTableColumns<Api.Log.RequestLog> = [
  { title: 'ID', key: 'id', width: 60 },
  { title: 'API Key', key: 'user_api_key', width: 120, ellipsis: { tooltip: true } },
  { title: '渠道', key: 'channel_id', width: 60 },
  { title: '模型', key: 'model' },
  { title: '端点', key: 'endpoint', width: 120 },
  { title: '状态码', key: 'status_code', width: 80, render: row => {
    const type = row.status_code < 300 ? 'success' : row.status_code < 400 ? 'warning' : 'error';
    return h(NTag, { type }, { default: () => row.status_code });
  }},
  { title: 'Prompt', key: 'prompt_tokens', width: 70 },
  { title: 'Completion', key: 'completion_tokens', width: 100 },
  { title: 'Total', key: 'total_tokens', width: 70 },
  { title: '耗时(ms)', key: 'elapsed_ms', width: 80 },
  { title: '时间', key: 'created_at', width: 180, render: row => new Date(row.created_at).toLocaleString() },
  {
    title: '错误',
    key: 'error_message',
    width: 100,
    render: row => row.error_message ? h(NTag, { type: 'error', size: 'small' }, { default: () => row.error_message!.substring(0, 20) }) : null
  },
  {
    title: '操作',
    key: 'actions',
    width: 80,
    render: row => h(NButton, { size: 'small', onClick: () => handleViewDetail(row) }, { default: () => '详情' })
  }
];

async function loadModelOptions() {
  const { data } = await fetchModelList(1, 1000);
  if (data) {
    const items = Array.isArray(data.items) ? data.items : (Array.isArray(data) ? data : []);
    modelOptions.value = [
      { label: '全部', value: '' },
      ...items.map((m: any) => ({ label: `${m.proxy_name} (${m.source_name})`, value: m.proxy_name }))
    ];
  }
}

async function loadData() {
  loading.value = true;
  const filterParams: { model?: string; status_code?: string } = {};
  if (filterModel.value) filterParams.model = filterModel.value;
  if (filterStatus.value) filterParams.status_code = filterStatus.value;

  const { data, error } = await fetchLogList(pagination.value.page, pagination.value.pageSize, filterParams);
  if (!error && data) {
    logs.value = data as Api.Log.RequestLog[];
    pagination.value.itemCount = data.length;
  }
  loading.value = false;
}

function handleFilter() {
  pagination.value.page = 1;
  loadData();
}

function handleReset() {
  filterModel.value = '';
  filterStatus.value = '';
  filterDateRange.value = null;
  pagination.value.page = 1;
  loadData();
}

function handleViewDetail(row: Api.Log.RequestLog) {
  selectedLog.value = row;
  showDrawer.value = true;
}

async function handleExport() {
  try {
    const response = await exportLogsCsv();
    if (!response.ok) {
      message.error('导出失败');
      return;
    }
    const blob = await response.blob();
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.download = `request_logs_${new Date().toISOString().slice(0, 10)}.csv`;
    link.click();
    URL.revokeObjectURL(url);
    message.success('导出成功');
  } catch {
    message.error('导出失败');
  }
}

onMounted(() => {
  loadData();
  loadModelOptions();
});
</script>

<template>
  <NSpace vertical :size="16">
    <NCard :bordered="false" class="card-wrapper">
      <template #header>
        <NSpace justify="space-between" align="center">
          <span>请求日志</span>
          <NButton type="primary" @click="handleExport">导出 CSV</NButton>
        </NSpace>
      </template>

      <!-- Filter bar -->
      <NGrid :x-gap="12" :y-gap="8" responsive="screen" item-responsive style="margin-bottom: 16px">
        <NGi span="24 s:8 m:5">
          <NSelect
            v-model:value="filterModel"
            :options="modelOptions"
            placeholder="按模型筛选"
            clearable
            filterable
          />
        </NGi>
        <NGi span="24 s:8 m:5">
          <NSelect
            v-model:value="filterStatus"
            :options="statusOptions"
            placeholder="按状态码筛选"
          />
        </NGi>
        <NGi span="24 s:8 m:6">
          <NDatePicker
            v-model:value="filterDateRange"
            type="daterange"
            placeholder="选择日期范围"
            clearable
          />
        </NGi>
        <NGi span="24 s:24 m:8">
          <NSpace>
            <NButton type="primary" @click="handleFilter">筛选</NButton>
            <NButton @click="handleReset">重置</NButton>
          </NSpace>
        </NGi>
      </NGrid>

      <NDataTable
        :columns="columns"
        :data="logs"
        :loading="loading"
        :pagination="pagination"
        :row-key="(row: Api.Log.RequestLog) => row.id"
        :max-height="600"
      />
    </NCard>

    <NDrawer v-model:show="showDrawer" :width="800" placement="right">
      <NDrawerContent title="请求详情" closable>
        <template v-if="selectedLog">
          <NDescriptions :column="2" bordered>
            <NDescriptionsItem label="ID">{{ selectedLog.id }}</NDescriptionsItem>
            <NDescriptionsItem label="API Key">{{ selectedLog.user_api_key }}</NDescriptionsItem>
            <NDescriptionsItem label="渠道ID">{{ selectedLog.channel_id }}</NDescriptionsItem>
            <NDescriptionsItem label="模型">{{ selectedLog.model }}</NDescriptionsItem>
            <NDescriptionsItem label="端点">{{ selectedLog.endpoint }}</NDescriptionsItem>
            <NDescriptionsItem label="状态码">
              <NTag :type="selectedLog.status_code < 300 ? 'success' : selectedLog.status_code < 400 ? 'warning' : 'error'">
                {{ selectedLog.status_code }}
              </NTag>
            </NDescriptionsItem>
            <NDescriptionsItem label="Prompt Tokens">{{ selectedLog.prompt_tokens }}</NDescriptionsItem>
            <NDescriptionsItem label="Completion Tokens">{{ selectedLog.completion_tokens }}</NDescriptionsItem>
            <NDescriptionsItem label="Total Tokens">{{ selectedLog.total_tokens }}</NDescriptionsItem>
            <NDescriptionsItem label="耗时(ms)">{{ selectedLog.elapsed_ms }}</NDescriptionsItem>
            <NDescriptionsItem label="时间">{{ new Date(selectedLog.created_at).toLocaleString() }}</NDescriptionsItem>
            <NDescriptionsItem label="错误信息" :span="2">
              <span v-if="selectedLog.error_message">{{ selectedLog.error_message }}</span>
              <span v-else style="color: #999">无</span>
            </NDescriptionsItem>
          </NDescriptions>

          <div v-if="selectedLog.request_body" style="margin-top: 16px">
            <h4 style="margin-bottom: 8px">请求体</h4>
            <NCode :code="typeof selectedLog.request_body === 'string' ? selectedLog.request_body : JSON.stringify(selectedLog.request_body, null, 2)" language="json" word-wrap />
          </div>

          <div v-if="selectedLog.response_body" style="margin-top: 16px">
            <h4 style="margin-bottom: 8px">响应体</h4>
            <NCode :code="typeof selectedLog.response_body === 'string' ? selectedLog.response_body : JSON.stringify(selectedLog.response_body, null, 2)" language="json" word-wrap />
          </div>
        </template>
      </NDrawerContent>
    </NDrawer>
  </NSpace>
</template>

<style scoped></style>
