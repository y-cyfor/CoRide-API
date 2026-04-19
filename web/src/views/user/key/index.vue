<script setup lang="ts">
import { ref, h, onMounted } from 'vue';
import { NButton, NTag, NSpace, NModal, NForm, NFormItem, NInput, NSelect, useDialog, useMessage, NCard, NDataTable, NInputNumber } from 'naive-ui';
import type { DataTableColumns } from 'naive-ui';
import { fetchUserKeyList, fetchCreateUserKey, fetchUpdateUserKey, fetchDeleteUserKey, fetchModelList } from '@/service/api';

const loading = ref(false);
const keys = ref<Api.UserKey.UserKey[]>([]);
const dialog = useDialog();
const message = useMessage();
const isEdit = ref(false);
const editingId = ref<number | null>(null);

const showModal = ref(false);
const formModel = ref({
  name: '',
  enabled_models: [] as string[]
});

const modelOptions = ref<{ label: string; value: string }[]>([]);

const columns: DataTableColumns<Api.UserKey.UserKey> = [
  { title: 'ID', key: 'id', width: 60 },
  { title: '名称', key: 'name', width: 120, render: row => row.name || '-' },
  { title: 'Key', key: 'key_value', width: 280, render: row => {
    const masked = row.key_value.substring(0, 10) + '****' + row.key_value.substring(row.key_value.length - 4);
    return h(NSpace, { align: 'center' }, {
      default: () => [
        h('code', { style: 'font-size: 12px' }, masked),
        h(NButton, { size: 'tiny', text: true, onClick: () => navigator.clipboard.writeText(row.key_value) }, { default: () => '复制' })
      ]
    });
  }},
  { title: '可访问模型', key: 'enabled_models', ellipsis: { tooltip: true }, render: row => row.enabled_models || '默认' },
  { title: '状态', key: 'status', width: 80, render: row => h(NTag, { type: row.status === 'active' ? 'success' : 'default' }, { default: () => row.status === 'active' ? '启用' : '禁用' }) },
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

async function loadModelOptions() {
  const { data } = await fetchModelList(1, 200);
  if (data) {
    const items = Array.isArray(data.items) ? data.items : (Array.isArray(data) ? data : []);
    modelOptions.value = items.map((m: any) => ({ label: m.proxy_name, value: m.proxy_name }));
  }
}

async function loadData() {
  loading.value = true;
  const { data, error } = await fetchUserKeyList();
  if (!error && data) {
    keys.value = data;
  }
  loading.value = false;
}

function handleCreate() {
  isEdit.value = false;
  editingId.value = null;
  formModel.value = { name: '', enabled_models: [] };
  showModal.value = true;
}

function handleEdit(row: Api.UserKey.UserKey) {
  isEdit.value = true;
  editingId.value = row.id;
  formModel.value = {
    name: row.name || '',
    enabled_models: row.enabled_models ? JSON.parse(row.enabled_models) : []
  };
  showModal.value = true;
}

async function handleSave() {
  const enabledModels = formModel.value.enabled_models.length > 0
    ? JSON.stringify(formModel.value.enabled_models)
    : undefined;

  let error: any;
  if (isEdit.value && editingId.value) {
    const res = await fetchUpdateUserKey(editingId.value, {
      name: formModel.value.name || undefined,
      enabled_models: enabledModels
    });
    error = res.error;
  } else {
    const res = await fetchCreateUserKey({
      name: formModel.value.name || undefined,
      enabled_models: enabledModels
    });
    error = res.error;
  }

  if (!error) {
    message.success(isEdit.value ? 'Key 已更新' : 'Key 创建成功');
    showModal.value = false;
    await loadData();
  }
}

async function handleDelete(row: Api.UserKey.UserKey) {
  dialog.warning({
    title: '确认删除',
    content: `确定要删除 Key "${row.name || row.key_value.substring(0, 10)}" 吗？`,
    positiveText: '确定',
    negativeText: '取消',
    onPositiveClick: async () => {
      const { error } = await fetchDeleteUserKey(row.id);
      if (!error) {
        message.success('Key 已删除');
        await loadData();
      }
    }
  });
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
          <span>Key 管理</span>
          <NButton type="primary" @click="handleCreate">创建 Key</NButton>
        </NSpace>
      </template>

      <NDataTable
        :columns="columns"
        :data="keys"
        :loading="loading"
        :row-key="(row: Api.UserKey.UserKey) => row.id"
      />
    </NCard>

    <NModal v-model:show="showModal" preset="card" :title="isEdit ? '编辑 Key' : '创建 Key'" style="width: 600px">
      <NForm :model="formModel" label-placement="left" label-width="100">
        <NFormItem label="名称">
          <NInput v-model:value="formModel.name" placeholder="Key 名称（可选）" />
        </NFormItem>
        <NFormItem label="可访问模型">
          <NSelect
            v-model:value="formModel.enabled_models"
            :options="modelOptions"
            multiple
            filterable
            placeholder="留空则使用用户默认模型权限"
          />
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
