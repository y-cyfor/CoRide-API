<script setup lang="ts">
import { ref, h, computed, onMounted } from 'vue';
import { NButton, NTag, NSpace, NModal, NForm, NFormItem, NInput, NSelect, NSwitch, useDialog, useMessage } from 'naive-ui';
import { fetchModelList, fetchCreateModel, fetchUpdateModel, fetchDeleteModel, fetchChannelList } from '@/service/api';
import type { DataTableColumns } from 'naive-ui';

const loading = ref(false);
const models = ref<Api.Model.Model[]>([]);
const channels = ref<{ id: number; name: string }[]>([]);
const dialog = useDialog();
const message = useMessage();
const isEdit = ref(false);
const editingId = ref<number | null>(null);

const showModal = ref(false);
const formModel = ref<Partial<Api.Model.CreateModelParams> & { enabled?: boolean }>({
  channel_id: 0,
  source_name: '',
  proxy_name: '',
  enabled: true
});

const channelOptions = ref<{ label: string; value: number }[]>([]);

interface TreeNode {
  id: number;
  rowKey: string;
  channel_id: number | string;
  channel_name: string;
  source_name: string;
  proxy_name: string;
  enabled: boolean;
  created_at: string;
  children?: Array<{
    id: number;
    rowKey: string;
    channel_id: string;
    channel_name: string;
    source_name: string;
    proxy_name: string;
    enabled: boolean;
    created_at: string;
  }>;
}

const treeData = computed<TreeNode[]>(() => {
  const channelMap = new Map<number, { id: number; name: string }>();
  for (const c of channels.value) {
    channelMap.set(c.id, c);
  }

  // Group models by channel
  const grouped = new Map<number, Api.Model.Model[]>();
  for (const m of models.value) {
    if (!grouped.has(m.channel_id)) {
      grouped.set(m.channel_id, []);
    }
    grouped.get(m.channel_id)!.push(m);
  }

  const result: TreeNode[] = [];
  // Sort channels by id to show in order
  const sortedChannelIds = [...grouped.keys()].sort((a, b) => a - b);

  for (const channelId of sortedChannelIds) {
    const ch = channelMap.get(channelId);
    const modelsInChannel = grouped.get(channelId) || [];

    const node: TreeNode = {
      id: channelId,
      rowKey: `ch_${channelId}`,
      channel_id: channelId,
      channel_name: ch?.name || '未知',
      source_name: '-',
      proxy_name: '-',
      enabled: true,
      created_at: '',
      children: modelsInChannel.map(m => ({
        id: m.id,
        rowKey: `model_${m.id}`,
        channel_id: '-',
        channel_name: '-',
        source_name: m.source_name,
        proxy_name: m.proxy_name,
        enabled: m.enabled,
        created_at: m.created_at
      }))
    };
    result.push(node);
  }

  return result;
});

const columns: DataTableColumns<TreeNode> = [
  { title: '渠道ID', key: 'channel_id', width: 80 },
  { title: '渠道', key: 'channel_name', width: 120 },
  { title: 'ID', key: 'id', width: 60 },
  { title: '源名称', key: 'source_name' },
  { title: '代理名称', key: 'proxy_name' },
  { title: '启用', key: 'enabled', width: 80, render: row => {
    if (row.source_name === '-') return null;
    return h(NTag, { type: row.enabled ? 'success' : 'default' }, { default: () => row.enabled ? '是' : '否' });
  }},
  { title: '创建时间', key: 'created_at', width: 180, render: row => {
    if (!row.created_at) return null;
    return new Date(row.created_at).toLocaleString();
  }},
  {
    title: '操作',
    key: 'actions',
    width: 160,
    render: row => {
      if (row.source_name === '-') return null;
      return h(NSpace, {}, {
        default: () => [
          h(NButton, { size: 'small', onClick: () => handleEditByModelId(row.id) }, { default: () => '编辑' }),
          h(NButton, { size: 'small', type: 'error', onClick: () => handleDeleteByModelId(row.id) }, { default: () => '删除' })
        ]
      });
    }
  }
];

// Keep a flat map for edit/delete by actual model id
const modelMap = computed(() => {
  const map = new Map<number, Api.Model.Model>();
  for (const m of models.value) {
    map.set(m.id, m);
  }
  return map;
});

function handleEditByModelId(modelId: number) {
  const model = modelMap.value.get(modelId);
  if (model) handleEdit(model);
}

function handleDeleteByModelId(modelId: number) {
  const model = modelMap.value.get(modelId);
  if (model) handleDelete(model);
}

async function loadData() {
  loading.value = true;
  const { data, error } = await fetchModelList(1, 100);
  if (!error && data) {
    const items = (data as any).items || (Array.isArray(data) ? data : []);
    models.value = items;
  }
  loading.value = false;
}

async function loadChannels() {
  const { data } = await fetchChannelList(1, 100);
  if (data) {
    const items = (data as any).items || data;
    const list = Array.isArray(items) ? items : [];
    channelOptions.value = list.map((c: any) => ({ label: c.name, value: c.id }));
    channels.value = list.map((c: any) => ({ id: c.id, name: c.name }));
  }
}

function handleCreate() {
  isEdit.value = false;
  editingId.value = null;
  formModel.value = {
    channel_id: 0,
    source_name: '',
    proxy_name: '',
    enabled: true
  };
  showModal.value = true;
}

function handleEdit(row: Api.Model.Model) {
  isEdit.value = true;
  editingId.value = row.id;
  formModel.value = {
    channel_id: row.channel_id,
    source_name: row.source_name,
    proxy_name: row.proxy_name,
    enabled: row.enabled
  };
  showModal.value = true;
}

async function handleSave() {
  if (!formModel.value.proxy_name) {
    message.warning('请填写代理名称');
    return;
  }

  let error: any;
  if (isEdit.value && editingId.value) {
    const res = await fetchUpdateModel(editingId.value, formModel.value);
    error = res.error;
  } else {
    if (!formModel.value.channel_id) {
      message.warning('请选择渠道');
      return;
    }
    const res = await fetchCreateModel(formModel.value as Api.Model.CreateModelParams);
    error = res.error;
  }

  if (!error) {
    message.success(isEdit.value ? '模型已更新' : '模型创建成功');
    showModal.value = false;
    await loadData();
  }
}

async function handleDelete(row: Api.Model.Model) {
  dialog.warning({
    title: '确认删除',
    content: `确定要删除模型 "${row.proxy_name}" 吗？`,
    positiveText: '确定',
    negativeText: '取消',
    onPositiveClick: async () => {
      const { error } = await fetchDeleteModel(row.id);
      if (!error) {
        message.success('模型已删除');
        await loadData();
      }
    }
  });
}

onMounted(() => {
  loadData();
  loadChannels();
});
</script>

<template>
  <NSpace vertical :size="16">
    <NCard :bordered="false" class="card-wrapper">
      <template #header>
        <NSpace justify="space-between" align="center">
          <span>模型管理</span>
          <NButton type="primary" @click="handleCreate">创建模型</NButton>
        </NSpace>
      </template>

      <NDataTable
        :columns="columns"
        :data="treeData"
        :loading="loading"
        :row-key="(row: TreeNode) => row.rowKey"
        :children-key="'children'"
        :pagination="false"
        :default-expand-all="true"
      />
    </NCard>

    <NModal v-model:show="showModal" preset="card" :title="isEdit ? '编辑模型' : '创建模型'" style="width: 500px">
      <NForm :model="formModel" label-placement="left" label-width="80">
        <NFormItem v-if="!isEdit" label="渠道">
          <NSelect v-model:value="formModel.channel_id" :options="channelOptions" placeholder="选择渠道" />
        </NFormItem>
        <NFormItem label="源名称">
          <NInput v-model:value="formModel.source_name" placeholder="上游模型名称" />
        </NFormItem>
        <NFormItem label="代理名称">
          <NInput v-model:value="formModel.proxy_name" placeholder="对外展示的模型名称" />
        </NFormItem>
        <NFormItem label="启用">
          <NSwitch v-model:value="formModel.enabled">
            <template #checked>是</template>
            <template #unchecked>否</template>
          </NSwitch>
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
