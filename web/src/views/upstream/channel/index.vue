<script setup lang="ts">
import { ref, h, onMounted } from 'vue';
import { NButton, NTag, NSpace, NModal, NForm, NFormItem, NInput, NSelect, NInputNumber, NProgress, NSwitch, useDialog, useMessage } from 'naive-ui';
import { fetchChannelList, fetchCreateChannel, fetchDeleteChannel, fetchUpdateChannel, fetchTestChannel } from '@/service/api';
import type { DataTableColumns } from 'naive-ui';

const loading = ref(false);
const channels = ref<Api.Channel.Channel[]>([]);
const dialog = useDialog();
const message = useMessage();
const isEdit = ref(false);
const editingId = ref<number | null>(null);

const pagination = ref({
  page: 1,
  pageSize: 20,
  itemCount: 0,
  showSizePicker: true,
  pageSizes: [10, 20, 50],
  prefix: ({ itemCount }: { itemCount: number }) => `共 ${itemCount} 条`,
  onChange: (page: number) => {
    pagination.value.page = page;
    loadData();
  },
  onUpdatePageSize: (pageSize: number) => {
    pagination.value.pageSize = pageSize;
    pagination.value.page = 1;
    loadData();
  }
});

const showModal = ref(false);
const formModel = ref<Partial<Api.Channel.CreateChannelParams>>({
  name: '',
  type: 'openai',
  base_url: '',
  api_keys: '',
  weight: 1,
  timeout: 300000,
  retry_count: 1
});
const channelEnabled = ref(true);

const typeOptions = [
  { label: 'OpenAI兼容', value: 'openai' },
  { label: 'Anthropic兼容', value: 'anthropic' }
];

function formatTokenNum(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return n.toLocaleString();
}

const columns: DataTableColumns<Api.Channel.Channel> = [
  { title: 'ID', key: 'id', width: 60 },
  { title: '名称', key: 'name' },
  { title: '类型', key: 'type', width: 100, render: row => h(NTag, { type: row.type === 'anthropic' ? 'warning' : 'info' }, { default: () => row.type }) },
  { title: 'BaseURL', key: 'base_url', ellipsis: { tooltip: true } },
  { title: '模型数', key: 'model_count', width: 70, render: row => (row as any).model_count || 0 },
  { title: '应用伪装', key: 'app_profile_name', width: 100, render: row => (row as any).app_profile_name ? h(NTag, { type: 'default', size: 'small' }, { default: () => (row as any).app_profile_name }) : h('span', { style: 'color: #999' }, '无') },
  { title: '权重', key: 'weight', width: 60 },
  { title: '状态', key: 'status', width: 80, render: row => h(NTag, { type: row.status === 'active' ? 'success' : 'default' }, { default: () => row.status }) },
  {
    title: '健康',
    key: 'consecutive_failures',
    width: 80,
    render: row => {
      const failures = row.consecutive_failures || 0;
      if (row.status !== 'active') return h(NTag, { type: 'default', size: 'small' }, { default: () => '-' });
      if (failures === 0) return h(NTag, { type: 'success', size: 'small' }, { default: () => '正常' });
      if (failures < 3) return h(NTag, { type: 'warning', size: 'small' }, { default: () => `${failures}次` });
      return h(NTag, { type: 'error', size: 'small' }, { default: () => '异常' });
    }
  },
  {
    title: '累计用量',
    key: 'total_usage',
    width: 140,
    render: row => {
      const stats = (row as any).stats;
      const reqs = stats?.total_requests ?? 0;
      const toks = stats?.total_tokens ?? 0;
      return h('div', { style: 'font-size: 12px' }, [
        h('div', null, `${reqs.toLocaleString()} 次`),
        h('div', { style: 'color: #999; font-size: 11px' }, `${formatTokenNum(toks)} tokens`)
      ]);
    }
  },
  {
    title: '今日用量',
    key: 'today_usage',
    width: 140,
    render: row => {
      const stats = (row as any).stats;
      const reqs = stats?.today_requests ?? 0;
      const toks = stats?.today_tokens ?? 0;
      return h('div', { style: 'font-size: 12px' }, [
        h('div', null, `${reqs.toLocaleString()} 次`),
        h('div', { style: 'color: #999; font-size: 11px' }, `${formatTokenNum(toks)} tokens`)
      ]);
    }
  },
  {
    title: '配额进度',
    key: 'quota_used',
    width: 180,
    render: row => {
      if (!row.quota_type || !row.quota_limit || row.quota_limit <= 0) {
        return h('span', { style: 'color: #999; font-size: 12px' }, '无限制');
      }
      const used = row.quota_used || 0;
      const total = row.quota_limit;
      const percent = Math.round((used / total) * 100);
      const color = percent >= 90 ? '#e80c47' : percent >= 70 ? '#f5a623' : '#18a058';

      // Cycle label
      const cycleLabels: Record<string, string> = {
        hourly: '次/时', daily: '次/天', weekly: '次/周', monthly: '次/月', permanent: '次'
      };
      const unit = row.quota_type === 'tokens' ? 'tokens' : (cycleLabels[row.quota_cycle || 'permanent'] || '次');
      const label = `${used}/${total} ${unit}`;

      return h('div', { style: 'width: 100%' }, [
        h('div', { style: 'font-size: 11px; margin-bottom: 2px; color: #666' }, label),
        h(NProgress, {
          percentage: Math.min(percent, 100),
          height: 6,
          color,
          showIndicator: false
        })
      ]);
    }
  },
  {
    title: '操作',
    key: 'actions',
    width: 200,
    render: row => h(NSpace, {}, {
      default: () => [
        h(NButton, { size: 'small', onClick: () => handleEdit(row) }, { default: () => '编辑' }),
        h(NButton, { size: 'small', onClick: () => handleTest(row) }, { default: () => '测试' }),
        h(NButton, { size: 'small', type: 'error', onClick: () => handleDelete(row) }, { default: () => '删除' })
      ]
    })
  }
];

async function loadData() {
  loading.value = true;
  const { data, error } = await fetchChannelList(pagination.value.page, pagination.value.pageSize);
  if (!error && data) {
    // Backend returns { items: [...], total: N }
    if (data.items && data.total !== undefined) {
      channels.value = data.items;
      pagination.value.itemCount = data.total;
    } else {
      // Fallback for old API format
      channels.value = data;
      pagination.value.itemCount = data.length;
    }
  }
  loading.value = false;
}

function handleCreate() {
  isEdit.value = false;
  editingId.value = null;
  formModel.value = {
    name: '', type: 'openai', base_url: '', api_keys: '[]',
    weight: 1, timeout: 300000, retry_count: 1
  };
  channelEnabled.value = true;
  showModal.value = true;
}

function handleEdit(row: Api.Channel.Channel) {
  isEdit.value = true;
  editingId.value = row.id;
  formModel.value = {
    name: row.name,
    type: row.type,
    base_url: row.base_url,
    api_keys: row.api_keys,
    weight: row.weight,
    timeout: row.timeout,
    retry_count: row.retry_count,
    quota_type: row.quota_type,
    quota_limit: row.quota_limit,
    quota_cycle: row.quota_cycle
  };
  channelEnabled.value = row.status === 'active';
  showModal.value = true;
}

async function handleSave() {
  if (!formModel.value.name || !formModel.value.base_url) {
    message.warning('请填写名称和BaseURL');
    return;
  }

  // Process api_keys: convert user-friendly input to JSON array string
  const keysRaw = formModel.value.api_keys || '';
  let apiKeysJson: string;
  try {
    const parsed = JSON.parse(keysRaw);
    if (Array.isArray(parsed)) {
      apiKeysJson = keysRaw;
    } else {
      apiKeysJson = JSON.stringify([keysRaw]);
    }
  } catch {
    // Not valid JSON — treat entire input as a single key
    apiKeysJson = JSON.stringify([keysRaw]);
  }

  const payload: Record<string, any> = {
    ...formModel.value,
    api_keys: apiKeysJson,
  };
  if (!isEdit.value) {
    // new channel — no status field needed
  } else {
    payload.status = channelEnabled.value ? 'active' : 'disabled';
  }

  let error: any;
  if (isEdit.value && editingId.value) {
    const res = await fetchUpdateChannel(editingId.value, payload);
    error = res.error;
  } else {
    const res = await fetchCreateChannel(payload as Api.Channel.CreateChannelParams);
    error = res.error;
  }

  if (!error) {
    message.success(isEdit.value ? '渠道已更新' : '渠道创建成功');
    showModal.value = false;
    await loadData();
  }
}

async function handleTest(row: Api.Channel.Channel) {
  const loadingMsg = message.loading('正在测试...', { duration: 0 });
  try {
    const { data, error } = await fetchTestChannel(row.id);
    loadingMsg.destroy();
    if (!error) {
      if (data?.status === 'ok') {
        message.success(`测试结果: 渠道连通正常 | 耗时 ${data.latency_ms}ms (HTTP ${data.http_status})`);
      } else if (data?.status === 'warning') {
        const httpStatus = data.http_status;
        // Special handling for 405 — /models endpoint not supported
        if (httpStatus === 405) {
          message.warning('该供应商不支持 /models 端点（部分 codingplan 版本未开放此接口），渠道配置本身是有效的，可直接绑定模型后测试');
        } else {
          message.warning(data.message || `渠道返回 HTTP ${httpStatus}`);
        }
      } else {
        message.info(`测试结果: ${data?.status || '未知'}`);
      }
    }
  } catch {
    loadingMsg.destroy();
  }
}

async function handleDelete(row: Api.Channel.Channel) {
  dialog.warning({
    title: '确认删除',
    content: `确定要删除渠道 "${row.name}" 吗？`,
    positiveText: '确定',
    negativeText: '取消',
    onPositiveClick: async () => {
      const { error } = await fetchDeleteChannel(row.id);
      if (!error) {
        message.success('渠道已删除');
        await loadData();
      }
    }
  });
}

onMounted(() => {
  loadData();
});
</script>

<template>
  <NSpace vertical :size="16">
    <NCard :bordered="false" class="card-wrapper">
      <template #header>
        <NSpace justify="space-between" align="center">
          <span>渠道管理</span>
          <NButton type="primary" @click="handleCreate">创建渠道</NButton>
        </NSpace>
      </template>

      <NDataTable
        :columns="columns"
        :data="channels"
        :loading="loading"
        :pagination="pagination"
        :remote="true"
        :row-key="(row: Api.Channel.Channel) => row.id"
      />
    </NCard>

    <NModal v-model:show="showModal" preset="card" :title="isEdit ? '编辑渠道' : '创建渠道'" style="width: 600px">
      <NForm :model="formModel" label-placement="left" label-width="100">
        <NFormItem label="名称">
          <NInput v-model:value="formModel.name" placeholder="渠道名称" />
        </NFormItem>
        <NFormItem label="类型">
          <NSelect v-model:value="formModel.type" :options="typeOptions" />
        </NFormItem>
        <NFormItem label="BaseURL">
          <NInput v-model:value="formModel.base_url" placeholder="https://api.openai.com/v1" />
        </NFormItem>
        <NFormItem label="API Keys">
          <NInput v-model:value="formModel.api_keys" type="textarea" placeholder="sk-xxxxx（多个 Key 用逗号分隔）" :rows="3" />
        </NFormItem>
        <NFormItem label="权重">
          <NInputNumber v-model:value="formModel.weight" :min="1" />
        </NFormItem>
        <NFormItem label="超时(ms)">
          <NInputNumber v-model:value="formModel.timeout" :min="1000" :step="1000" />
        </NFormItem>
        <NFormItem label="重试次数">
          <NInputNumber v-model:value="formModel.retry_count" :min="0" :max="5" />
        </NFormItem>
        <NFormItem label="状态" v-if="isEdit">
          <NSwitch v-model:value="channelEnabled">
            <template #checked>启用</template>
            <template #unchecked>禁用</template>
          </NSwitch>
        </NFormItem>
        <NFormItem label="配额类型">
          <NSelect v-model:value="formModel.quota_type" :options="[
            { label: '无限制', value: null },
            { label: '请求数', value: 'requests' },
            { label: 'Token数', value: 'tokens' }
          ]" placeholder="选择配额类型" clearable />
        </NFormItem>
        <NFormItem label="配额上限">
          <NInputNumber v-model:value="formModel.quota_limit" :min="0" placeholder="配额上限" />
        </NFormItem>
        <NFormItem label="配额周期">
          <NSelect v-model:value="formModel.quota_cycle" :options="[
            { label: '永久', value: 'permanent' },
            { label: '每小时', value: 'hourly' },
            { label: '每天', value: 'daily' },
            { label: '每周', value: 'weekly' },
            { label: '每月', value: 'monthly' }
          ]" placeholder="选择周期" clearable />
        </NFormItem>
      </NForm>
      <template #footer>
        <NSpace justify="end">
          <NButton @click="showModal = false">取消</NButton>
          <NButton type="primary" @click="handleSave">保存</NButton>
        </NSpace>
      </template>
    </NModal>
  </NSpace>
</template>

<style scoped></style>
