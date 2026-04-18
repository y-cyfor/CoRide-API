<script setup lang="ts">
import { ref, h, onMounted } from 'vue';
import { NButton, NTag, NSpace, NModal, NForm, NFormItem, NInput, NSelect, useDialog, useMessage } from 'naive-ui';
import { fetchQuotaList, fetchCreateQuota, fetchUpdateQuota, fetchDeleteQuota, fetchUserList } from '@/service/api';
import { fetchChannelList } from '@/service/api/channel';
import type { DataTableColumns } from 'naive-ui';

const loading = ref(false);
const quotas = ref<Api.Quota.Quota[]>([]);
const users = ref<Map<number, string>>(new Map());
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
const formModel = ref<Partial<Api.Quota.CreateQuotaParams & { total_limit: number }>>({
  quota_type: 'tokens',
  total_limit: 0,
  cycle: 'daily'
});

const userOptions = ref<{ label: string; value: number }[]>([]);
const channelOptions = ref<{ label: string; value: number | undefined }[]>([]);
const channels = ref<Map<number, string>>(new Map());

const quotaTypeOptions = [
  { label: '请求次数', value: 'requests' },
  { label: 'Token数量', value: 'tokens' }
];

const cycleOptions = [
  { label: '每小时', value: 'hourly' },
  { label: '每天', value: 'daily' },
  { label: '每周', value: 'weekly' },
  { label: '每月', value: 'monthly' },
  { label: '永久', value: 'permanent' }
];

async function loadUsers() {
  const { data } = await fetchUserList(1, 100);
  if (data) {
    userOptions.value = data.map(u => ({ label: u.username, value: u.id }));
    users.value = new Map(data.map((u: any) => [u.id, u.username]));
  }
}

async function loadChannels() {
  const { data } = await fetchChannelList(1, 100);
  if (data) {
    channelOptions.value = [{ label: '全部（用户级配额）', value: undefined }, ...data.map(c => ({ label: c.name, value: c.id }))];
    channels.value = new Map(data.map((c: Api.Channel.Channel) => [c.id, c.name]));
  }
}

const columns: DataTableColumns<Api.Quota.Quota> = [
  { title: 'ID', key: 'id', width: 60 },
  { title: '用户', key: 'username', width: 100, render: row => users.value.get(row.user_id) || `#${row.user_id}` },
  { title: '渠道', key: 'channel', width: 100, render: row => row.channel_id ? (channels.value.get(row.channel_id) || `#${row.channel_id}`) : '全部' },
  { title: '类型', key: 'quota_type', width: 80, render: row => h(NTag, { type: row.quota_type === 'tokens' ? 'warning' : 'info' }, { default: () => row.quota_type }) },
  { title: '总限额', key: 'total_limit', width: 100 },
  { title: '已使用', key: 'used', width: 80 },
  { title: '周期', key: 'cycle', width: 80, render: row => h(NTag, {}, { default: () => row.cycle }) },
  { title: '启用', key: 'enabled', width: 60, render: row => row.enabled ? '是' : '否' },
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
  const { data, error } = await fetchQuotaList(pagination.value.page, pagination.value.pageSize);
  if (!error && data) {
    quotas.value = data;
    pagination.value.itemCount = data.length;
  }
  loading.value = false;
}

function handleCreate() {
  isEdit.value = false;
  editingId.value = null;
  formModel.value = { user_id: undefined as unknown as number, quota_type: 'tokens', total_limit: 0, cycle: 'daily', channel_id: undefined };
  showModal.value = true;
}

function handleEdit(row: Api.Quota.Quota) {
  isEdit.value = true;
  editingId.value = row.id;
  formModel.value = {
    total_limit: row.total_limit
  };
  showModal.value = true;
}

async function handleSave() {
  let error: any;
  if (isEdit.value && editingId.value) {
    const res = await fetchUpdateQuota(editingId.value, { total_limit: formModel.value.total_limit });
    error = res.error;
  } else {
    if (!formModel.value.user_id) {
      message.warning('请选择用户');
      return;
    }
    const res = await fetchCreateQuota(formModel.value as Api.Quota.CreateQuotaParams);
    error = res.error;
  }

  if (!error) {
    message.success(isEdit.value ? '配额已更新' : '配额创建成功');
    showModal.value = false;
    await loadData();
  }
}

async function handleDelete(row: Api.Quota.Quota) {
  dialog.warning({
    title: '确认删除',
    content: `确定要删除配额 #${row.id} 吗？`,
    positiveText: '确定',
    negativeText: '取消',
    onPositiveClick: async () => {
      const { error } = await fetchDeleteQuota(row.id);
      if (!error) {
        message.success('配额已删除');
        await loadData();
      }
    }
  });
}

onMounted(() => {
  loadData();
  loadUsers();
  loadChannels();
});
</script>

<template>
  <NSpace vertical :size="16">
    <NCard :bordered="false" class="card-wrapper">
      <template #header>
        <NSpace justify="space-between" align="center">
          <span>配额管理</span>
          <NButton type="primary" @click="handleCreate">创建配额</NButton>
        </NSpace>
      </template>

      <NDataTable
        :columns="columns"
        :data="quotas"
        :loading="loading"
        :pagination="pagination"
        :row-key="(row: Api.Quota.Quota) => row.id"
      />
    </NCard>

    <NModal v-model:show="showModal" preset="card" :title="isEdit ? '编辑配额' : '创建配额'" style="width: 500px">
      <NForm :model="formModel" label-placement="left" label-width="80">
        <NFormItem v-if="!isEdit" label="用户">
          <NSelect v-model:value="formModel.user_id" :options="userOptions" filterable placeholder="搜索并选择用户" />
        </NFormItem>
        <NFormItem v-if="!isEdit" label="渠道">
          <NSelect v-model:value="formModel.channel_id" :options="channelOptions" placeholder="留空为用户级配额" />
        </NFormItem>
        <NFormItem v-if="!isEdit" label="类型">
          <NSelect v-model:value="formModel.quota_type" :options="quotaTypeOptions" />
        </NFormItem>
        <NFormItem label="总限额">
          <NInputNumber v-model:value="formModel.total_limit" :min="0" />
        </NFormItem>
        <NFormItem v-if="!isEdit" label="周期">
          <NSelect v-model:value="formModel.cycle" :options="cycleOptions" />
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
