<script setup lang="ts">
import { ref, h, onMounted } from 'vue';
import { NButton, NTag, NSpace, NModal, NForm, NFormItem, NInput, NInputNumber, NSwitch, useDialog, useMessage } from 'naive-ui';
import { fetchAppProfileList, fetchCreateAppProfile, fetchUpdateAppProfile, fetchDeleteAppProfile } from '@/service/api';
import type { DataTableColumns } from 'naive-ui';

const loading = ref(false);
const profiles = ref<Api.AppProfile.AppProfile[]>([]);
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
const formModel = ref<Partial<Api.AppProfile.CreateAppProfileParams>>({
  name: '',
  identifier: '',
  user_agent: '',
  extra_headers: '',
  description: ''
});

const identifierError = ref('');
const identifierRegex = /^[a-z][a-z0-9-]*$/;

function validateIdentifier(value: string) {
  if (!value) {
    identifierError.value = '标识符不能为空';
    return false;
  }
  if (!identifierRegex.test(value)) {
    identifierError.value = '标识符只能包含小写字母、数字和连字符，且必须以小写字母开头';
    return false;
  }
  identifierError.value = '';
  return true;
}

const columns: DataTableColumns<Api.AppProfile.AppProfile> = [
  { title: 'ID', key: 'id', width: 60 },
  { title: '名称', key: 'name' },
  { title: '标识符', key: 'identifier' },
  { title: 'User-Agent', key: 'user_agent', ellipsis: { tooltip: true } },
  { title: '系统', key: 'is_system', width: 60, render: row => h(NTag, { type: row.is_system ? 'warning' : 'default' }, { default: () => row.is_system ? '是' : '否' }) },
  { title: '启用', key: 'enabled', width: 60, render: row => row.enabled ? '是' : '否' },
  {
    title: '操作',
    key: 'actions',
    width: 160,
    render: row => row.is_system
      ? h(NTag, { type: 'default', size: 'small' }, { default: () => '系统' })
      : h(NSpace, {}, {
          default: () => [
            h(NButton, { size: 'small', onClick: () => handleEdit(row) }, { default: () => '编辑' }),
            h(NButton, { size: 'small', type: 'error', onClick: () => handleDelete(row) }, { default: () => '删除' })
          ]
        })
  }
];

async function loadData() {
  loading.value = true;
  const { data, error } = await fetchAppProfileList(pagination.value.page, pagination.value.pageSize);
  if (!error && data) {
    profiles.value = data;
    pagination.value.itemCount = data.length;
  }
  loading.value = false;
}

function handleCreate() {
  isEdit.value = false;
  editingId.value = null;
  formModel.value = { name: '', identifier: '', user_agent: '', extra_headers: '', description: '' };
  identifierError.value = '';
  showModal.value = true;
}

function handleEdit(row: Api.AppProfile.AppProfile) {
  if (row.is_system) {
    message.warning('系统预设不可编辑');
    return;
  }
  isEdit.value = true;
  editingId.value = row.id;
  formModel.value = {
    name: row.name,
    identifier: row.identifier,
    user_agent: row.user_agent,
    extra_headers: row.extra_headers,
    description: row.description
  };
  identifierError.value = '';
  showModal.value = true;
}

async function handleSave() {
  if (!formModel.value.name || !formModel.value.identifier || !formModel.value.user_agent) {
    message.warning('请填写必填项');
    return;
  }

  if (!validateIdentifier(formModel.value.identifier)) {
    message.warning('标识符格式不正确');
    return;
  }

  let error: any;
  if (isEdit.value && editingId.value) {
    const res = await fetchUpdateAppProfile(editingId.value, formModel.value);
    error = res.error;
  } else {
    const res = await fetchCreateAppProfile(formModel.value as Api.AppProfile.CreateAppProfileParams);
    error = res.error;
  }

  if (!error) {
    message.success(isEdit.value ? '预设已更新' : '预设创建成功');
    showModal.value = false;
    await loadData();
  }
}

async function handleDelete(row: Api.AppProfile.AppProfile) {
  if (row.is_system) {
    message.warning('系统预设不可删除');
    return;
  }
  dialog.warning({
    title: '确认删除',
    content: `确定要删除预设 "${row.name}" 吗？`,
    positiveText: '确定删除',
    negativeText: '取消',
    onPositiveClick: async () => {
      const { error } = await fetchDeleteAppProfile(row.id);
      if (!error) {
        message.success('预设已删除');
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
          <span>应用预设管理</span>
          <span style="color: #999; font-size: 12px;">配置可伪装的应用（UA + 请求头），供"应用方案"中的分流比例选择</span>
          <NButton type="primary" @click="handleCreate">创建预设</NButton>
        </NSpace>
      </template>

      <NDataTable
        :columns="columns"
        :data="profiles"
        :loading="loading"
        :pagination="pagination"
        :row-key="(row: Api.AppProfile.AppProfile) => row.id"
      />
    </NCard>

    <NModal v-model:show="showModal" preset="card" :title="isEdit ? '编辑预设' : '创建预设'" style="width: 600px">
      <NForm :model="formModel" label-placement="left" label-width="100">
        <NFormItem label="名称">
          <NInput v-model:value="formModel.name" placeholder="预设名称" />
        </NFormItem>
        <NFormItem label="标识符">
          <NInput v-model:value="formModel.identifier" placeholder="如: claude-code（小写字母、数字和连字符）" />
          <span v-if="identifierError" style="color: #e80c47; font-size: 12px; margin-top: 4px; display: block;">{{ identifierError }}</span>
        </NFormItem>
        <NFormItem label="User-Agent">
          <NInput v-model:value="formModel.user_agent" placeholder="如: claude-code-sdk/1.0" />
        </NFormItem>
        <NFormItem label="Extra Headers">
          <NInput v-model:value="formModel.extra_headers" type="textarea" placeholder='JSON对象: {"key":"value"}' :rows="3" />
        </NFormItem>
        <NFormItem label="描述">
          <NInput v-model:value="formModel.description" type="textarea" :rows="2" />
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
