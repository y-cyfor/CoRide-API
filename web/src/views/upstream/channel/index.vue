<script setup lang="ts">
import { ref, h, onMounted } from 'vue';
import { NButton, NTag, NSpace, NModal, NForm, NFormItem, NInput, NSelect, NInputNumber, NProgress, NCascader, NSwitch, useDialog, useMessage } from 'naive-ui';
import { fetchChannelList, fetchCreateChannel, fetchDeleteChannel, fetchUpdateChannel, fetchTestChannel, fetchCreateModel } from '@/service/api';
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
  itemCount: 0
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

// Supplier cascader options: Supplier -> Version -> Interface Type
interface SupplierOption {
  label: string;
  value: string;
  children?: Array<{
    label: string;
    value: string;
    children?: Array<{ label: string; value: string; url: string; type: 'openai' | 'anthropic' }>;
  }>;
}

const supplierOptions: SupplierOption[] = [
  {
    label: '阿里云 (通义千问)', value: 'aliyun',
    children: [
      {
        label: '标准版', value: 'standard',
        children: [
          { label: 'OpenAI 兼容', value: 'openai', url: 'https://dashscope.aliyuncs.com/compatible-mode/v1', type: 'openai' },
          { label: 'Anthropic 兼容', value: 'anthropic', url: 'https://dashscope.aliyuncs.com/compatible-mode/v1/apps/anthropic', type: 'anthropic' },
        ]
      },
      {
        label: 'CodingPlan 版', value: 'codingplan',
        children: [
          { label: 'OpenAI 兼容', value: 'openai', url: 'https://coding.dashscope.aliyuncs.com/v1', type: 'openai' },
          { label: 'Anthropic 兼容', value: 'anthropic', url: 'https://coding.dashscope.aliyuncs.com/apps/anthropic', type: 'anthropic' },
        ]
      },
    ]
  },
  {
    label: '智谱 AI', value: 'zhipu',
    children: [
      {
        label: '标准版', value: 'standard',
        children: [
          { label: 'OpenAI 兼容', value: 'openai', url: 'https://open.bigmodel.cn/api/paas/v4', type: 'openai' },
          { label: 'Anthropic 兼容', value: 'anthropic', url: 'https://open.bigmodel.cn/api/paas/v4/anthropic', type: 'anthropic' },
        ]
      },
      {
        label: 'CodingPlan 版', value: 'codingplan',
        children: [
          { label: 'OpenAI 兼容', value: 'openai', url: 'https://open.bigmodel.cn/api/paas/v4', type: 'openai' },
          { label: 'Anthropic 兼容', value: 'anthropic', url: 'https://open.bigmodel.cn/api/anthropic', type: 'anthropic' },
        ]
      },
    ]
  },
  {
    label: 'Kimi (月之暗面)', value: 'kimi',
    children: [
      {
        label: '标准版', value: 'standard',
        children: [
          { label: 'OpenAI 兼容', value: 'openai', url: 'https://api.moonshot.cn/v1', type: 'openai' },
          { label: 'Anthropic 兼容', value: 'anthropic', url: 'https://api.moonshot.cn/v1/anthropic', type: 'anthropic' },
        ]
      },
      {
        label: 'CodingPlan 版', value: 'codingplan',
        children: [
          { label: 'OpenAI 兼容', value: 'openai', url: 'https://api.kimi.com/coding/', type: 'openai' },
          { label: 'Anthropic 兼容', value: 'anthropic', url: 'https://api.kimi.com/coding/', type: 'anthropic' },
        ]
      },
    ]
  },
  {
    label: '小米', value: 'xiaomi',
    children: [
      {
        label: '标准版', value: 'standard',
        children: [
          { label: 'OpenAI 兼容', value: 'openai', url: 'https://api.miapi.xiaomi.com/v1', type: 'openai' },
          { label: 'Anthropic 兼容', value: 'anthropic', url: 'https://api.miapi.xiaomi.com/v1/anthropic', type: 'anthropic' },
        ]
      },
      {
        label: 'CodingPlan 版', value: 'codingplan',
        children: [
          { label: 'OpenAI 兼容', value: 'openai', url: 'https://token-plan-cn.xiaomimimo.com/v1', type: 'openai' },
          { label: 'Anthropic 兼容', value: 'anthropic', url: 'https://token-plan-cn.xiaomimimo.com/anthropic', type: 'anthropic' },
        ]
      },
    ]
  },
  {
    label: 'MiniMax', value: 'minimax',
    children: [
      {
        label: '标准版', value: 'standard',
        children: [
          { label: 'OpenAI 兼容', value: 'openai', url: 'https://api.minimaxi.com/v1', type: 'openai' },
          { label: 'Anthropic 兼容', value: 'anthropic', url: 'https://api.minimaxi.com/v1/anthropic', type: 'anthropic' },
        ]
      },
      {
        label: 'CodingPlan 版', value: 'codingplan',
        children: [
          { label: 'OpenAI 兼容', value: 'openai', url: 'https://api.minimaxi.com/v1/coding', type: 'openai' },
          { label: 'Anthropic 兼容', value: 'anthropic', url: 'https://api.minimaxi.com/v1/coding/anthropic', type: 'anthropic' },
        ]
      },
    ]
  },
  {
    label: 'OpenAI (官方)', value: 'openai',
    children: [
      { label: '标准版', value: 'standard', url: 'https://api.openai.com/v1', type: 'openai' },
    ]
  },
  {
    label: 'Anthropic (官方)', value: 'anthropic',
    children: [
      { label: '标准版', value: 'standard', url: 'https://api.anthropic.com/v1', type: 'anthropic' },
    ]
  },
];

// Model presets for each supplier - common models to import
const MODEL_PRESETS: Record<string, Array<{ source_name: string; proxy_name: string }>> = {
  'aliyun': [
    { source_name: 'qwen-turbo', proxy_name: 'qwen-turbo' },
    { source_name: 'qwen-plus', proxy_name: 'qwen-plus' },
    { source_name: 'qwen-max', proxy_name: 'qwen-max' },
    { source_name: 'qwen-long', proxy_name: 'qwen-long' },
  ],
  'zhipu': [
    { source_name: 'glm-4-plus', proxy_name: 'glm-4-plus' },
    { source_name: 'glm-4-flash', proxy_name: 'glm-4-flash' },
    { source_name: 'glm-4', proxy_name: 'glm-4' },
  ],
  'kimi': [
    { source_name: 'moonshot-v1-8k', proxy_name: 'moonshot-v1-8k' },
    { source_name: 'moonshot-v1-32k', proxy_name: 'moonshot-v1-32k' },
    { source_name: 'moonshot-v1-128k', proxy_name: 'moonshot-v1-128k' },
  ],
  'xiaomi': [
    { source_name: 'MiMo', proxy_name: 'MiMo' },
  ],
  'minimax': [
    { source_name: 'MiniMax-M2.1', proxy_name: 'MiniMax-M2.1' },
    { source_name: 'MiniMax-Text-01', proxy_name: 'MiniMax-Text-01' },
  ],
  'openai': [
    { source_name: 'gpt-4o', proxy_name: 'gpt-4o' },
    { source_name: 'gpt-4o-mini', proxy_name: 'gpt-4o-mini' },
    { source_name: 'gpt-4', proxy_name: 'gpt-4' },
  ],
  'anthropic': [
    { source_name: 'claude-sonnet-4-20250514', proxy_name: 'claude-sonnet-4' },
    { source_name: 'claude-opus-4-20250416', proxy_name: 'claude-opus-4' },
    { source_name: 'claude-haiku-4-20250324', proxy_name: 'claude-haiku-4' },
  ],
};

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

    // After creating a new channel, offer to import common models
    if (!isEdit.value && !editingId.value) {
      // Find the newly created channel
      const newChannel = channels.value[0];
      if (newChannel) {
        const supplier = Object.keys(MODEL_PRESETS).find(key =>
          newChannel.name.includes(MODEL_PRESETS[key][0]?.proxy_name || '')
        ) || findSupplierFromUrl(newChannel.base_url);
        if (supplier && MODEL_PRESETS[supplier]) {
          offerImportModels(newChannel.id, supplier);
        }
      }
    }
  }
}

function onSupplierSelect(value: string[], option: any[]) {
  // value is [supplier, version, interface_type]
  if (option.length >= 3) {
    const supplier = option[0];
    const version = option[1];
    const iface = option[option.length - 1];
    formModel.value.name = `${supplier.label} ${version.label}`;
    formModel.value.base_url = iface.url;
    formModel.value.type = iface.type;
  } else if (option.length === 2) {
    // Fallback for 2-level options (OpenAI/Anthropic official)
    const supplier = option[0];
    const child = option[1];
    formModel.value.name = supplier.label;
    formModel.value.base_url = child.url || '';
    formModel.value.type = child.type || 'openai';
  }
}

// Find supplier key from base_url
function findSupplierFromUrl(url: string): string | null {
  if (url.includes('dashscope')) return 'aliyun';
  if (url.includes('bigmodel')) return 'zhipu';
  if (url.includes('moonshot') || url.includes('kimi')) return 'kimi';
  if (url.includes('xiaoai') || url.includes('miapi') || url.includes('mimo')) return 'xiaomi';
  if (url.includes('minimax')) return 'minimax';
  if (url.includes('openai.com')) return 'openai';
  if (url.includes('anthropic.com')) return 'anthropic';
  return null;
}

// Offer to import common models after channel creation
function offerImportModels(channelId: number, supplier: string) {
  const presets = MODEL_PRESETS[supplier] || [];
  if (presets.length === 0) return;

  const modelNames = presets.map(p => p.proxy_name).join(', ');
  dialog.warning({
    title: '导入常见模型',
    content: `检测到您添加了${supplier === 'aliyun' ? '阿里云' : supplier === 'zhipu' ? '智谱' : supplier === 'kimi' ? 'Kimi' : supplier === 'xiaomi' ? '小米' : supplier === 'minimax' ? 'MiniMax' : supplier === 'openai' ? 'OpenAI' : 'Anthropic'}渠道，是否一键导入以下常见模型？\n\n${modelNames}`,
    positiveText: '导入',
    negativeText: '跳过',
    onPositiveClick: async () => {
      let successCount = 0;
      for (const m of presets) {
        const { error } = await fetchCreateModel({
          channel_id: channelId,
          source_name: m.source_name,
          proxy_name: m.proxy_name,
          enabled: true,
          is_default: false
        });
        if (!error) successCount++;
      }
      if (successCount > 0) {
        message.success(`已导入 ${successCount}/${presets.length} 个模型`);
        await loadData();
      }
    }
  });
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
        <NFormItem label="供应商预设">
          <NCascader
            :options="supplierOptions"
            placeholder="选择供应商 → 版本 → 自动填充"
            clearable
            expand-trigger="hover"
            @update:value="onSupplierSelect"
          />
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
