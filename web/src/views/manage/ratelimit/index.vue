<script setup lang="ts">
import { ref, h, onMounted, computed, watch } from 'vue';
import { NButton, NTag, NSpace, NModal, NForm, NFormItem, NInput, NInputNumber, NSelect, useDialog, useMessage } from 'naive-ui';
import { fetchRateLimitList, fetchCreateRateLimit, fetchUpdateRateLimit, fetchDeleteRateLimit, fetchUserList, fetchChannelList } from '@/service/api';
import type { DataTableColumns } from 'naive-ui';

const loading = ref(false);
const ratelimits = ref<Api.RateLimit.RateLimit[]>([]);
const dialog = useDialog();
const message = useMessage();
const isEdit = ref(false);
const editingId = ref<number | null>(null);

const showModal = ref(false);
const formModel = ref<Api.RateLimit.CreateRateLimitParams & { qps: number; concurrency: number; action: string }>({
  target_type: 'user',
  target_id: null,
  qps: 10,
  concurrency: 20,
  action: 'reject'
});

const targetTypeOptions = [
  { label: '用户', value: 'user' },
  { label: '渠道', value: 'channel' }
];

const actionOptions = [
  { label: '拒绝', value: 'reject' },
  { label: '排队', value: 'queue' }
];

// User options for dropdown
const userOptions = ref<{ label: string; value: number }[]>([]);
const userLoading = ref(false);

async function loadUserOptions() {
  userLoading.value = true;
  const { data, error } = await fetchUserList(1, 1000);
  if (!error && data) {
    const items = Array.isArray(data.items) ? data.items : (Array.isArray(data) ? data : []);
    userOptions.value = items.map((u: any) => ({
      label: `${u.username} (ID: ${u.id})`,
      value: u.id
    }));
  }
  userLoading.value = false;
}

// Channel options for dropdown
const channelOptions = ref<{ label: string; value: number }[]>([]);
const channelLoading = ref(false);

async function loadChannelOptions() {
  channelLoading.value = true;
  const { data, error } = await fetchChannelList(1, 1000);
  if (!error && data) {
    const items = Array.isArray(data.items) ? data.items : (Array.isArray(data) ? data : []);
    channelOptions.value = items.map((c: any) => ({
      label: `${c.name} (${c.type})`,
      value: c.id
    }));
  }
  channelLoading.value = false;
}

// Clear target_id when switching target type
watch(() => formModel.value.target_type, () => {
  formModel.value.target_id = null;
});

const columns: DataTableColumns<Api.RateLimit.RateLimit> = [
  { title: 'ID', key: 'id', width: 60 },
  { title: '目标类型', key: 'target_type', width: 100, render: row => h(NTag, { type: row.target_type === 'user' ? 'info' : 'warning' }, { default: () => row.target_type === 'user' ? '用户' : '渠道' }) },
  { title: '目标', key: 'target_id', width: 120, render: row => {
    if (row.target_type === 'user') {
      const user = userOptions.value.find(u => u.value === row.target_id);
      return user ? user.label : `用户 #${row.target_id}`;
    }
    const channel = channelOptions.value.find(c => c.value === row.target_id);
    return channel ? channel.label : `渠道 #${row.target_id}`;
  }},
  { title: 'QPS', key: 'qps', width: 80 },
  { title: '并发数', key: 'concurrency', width: 80 },
  { title: '动作', key: 'action', width: 80, render: row => h(NTag, { type: row.action === 'reject' ? 'error' : 'warning' }, { default: () => row.action }) },
  { title: '创建时间', key: 'created_at', width: 180, render: row => new Date(row.created_at).toLocaleString() },
  {
    title: '操作',
    key: 'actions',
    width: 160,
    render: row => h(NSpace, {}, {
      default: () => [
        h(NButton, { size: 'small', onClick: () => handleEdit(row) }, { default: () => '编辑' }),
        h(NButton, { size: 'small', type: 'error', onClick: () => handleDelete(row) }, { default: () => '删除' })
      ]
    })
  }
];

async function loadData() {
  loading.value = true;
  const { data, error } = await fetchRateLimitList();
  if (!error && data) {
    ratelimits.value = data;
  }
  loading.value = false;
}

function handleCreate() {
  isEdit.value = false;
  editingId.value = null;
  formModel.value = { target_type: 'user', target_id: null, qps: 10, concurrency: 20, action: 'reject' };
  showModal.value = true;
}

function handleEdit(row: Api.RateLimit.RateLimit) {
  isEdit.value = true;
  editingId.value = row.id;
  formModel.value = {
    target_type: row.target_type,
    target_id: row.target_id || null,
    qps: row.qps,
    concurrency: row.concurrency,
    action: row.action
  };
  showModal.value = true;
}

async function handleSave() {
  if (!formModel.value.target_id) {
    message.warning('请选择目标用户或渠道');
    return;
  }
  let error: any;
  if (isEdit.value && editingId.value) {
    const res = await fetchUpdateRateLimit(editingId.value, {
      qps: formModel.value.qps,
      concurrency: formModel.value.concurrency,
      action: formModel.value.action
    });
    error = res.error;
  } else {
    const res = await fetchCreateRateLimit(formModel.value);
    error = res.error;
  }

  if (!error) {
    message.success(isEdit.value ? '限流规则已更新' : '限流规则创建成功');
    showModal.value = false;
    await loadData();
  }
}

async function handleDelete(row: Api.RateLimit.RateLimit) {
  dialog.warning({
    title: '确认删除',
    content: `确定要删除限流规则 #${row.id} 吗？`,
    positiveText: '确定',
    negativeText: '取消',
    onPositiveClick: async () => {
      const { error } = await fetchDeleteRateLimit(row.id);
      if (!error) {
        message.success('限流规则已删除');
        await loadData();
      }
    }
  });
}

onMounted(() => {
  loadData();
  loadUserOptions();
  loadChannelOptions();
});
</script>

<template>
  <NSpace vertical :size="16">
    <NCard :bordered="false" class="card-wrapper">
      <template #header>
        <NSpace justify="space-between" align="center">
          <span>限流管理</span>
          <NButton type="primary" @click="handleCreate">创建规则</NButton>
        </NSpace>
      </template>

      <NDataTable
        :columns="columns"
        :data="ratelimits"
        :loading="loading"
        :row-key="(row: Api.RateLimit.RateLimit) => row.id"
      />
    </NCard>

    <NModal v-model:show="showModal" preset="card" :title="isEdit ? '编辑限流规则' : '创建限流规则'" style="width: 500px">
      <NForm :model="formModel" label-placement="left" label-width="80">
        <NFormItem v-if="!isEdit" label="目标类型">
          <NSelect v-model:value="formModel.target_type" :options="targetTypeOptions" />
        </NFormItem>
        <NFormItem v-if="!isEdit" label="目标用户" v-show="formModel.target_type === 'user'">
          <NSelect
            v-model:value="formModel.target_id"
            :options="userOptions"
            :loading="userLoading"
            placeholder="请选择用户"
            filterable
          />
        </NFormItem>
        <NFormItem v-if="!isEdit" label="目标渠道" v-show="formModel.target_type === 'channel'">
          <NSelect
            v-model:value="formModel.target_id"
            :options="channelOptions"
            :loading="channelLoading"
            placeholder="请选择渠道"
            filterable
          />
        </NFormItem>
        <NFormItem label="QPS">
          <NInputNumber v-model:value="formModel.qps" :min="1" />
        </NFormItem>
        <NFormItem label="并发数">
          <NInputNumber v-model:value="formModel.concurrency" :min="1" />
        </NFormItem>
        <NFormItem label="动作">
          <NSelect v-model:value="formModel.action" :options="actionOptions" />
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
