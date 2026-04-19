<script setup lang="ts">
import { ref, h, onMounted } from 'vue';
import { NButton, NTag, NSpace, NModal, NForm, NFormItem, NInput, NInputNumber, NSelect, NCascader, NProgress, useDialog, useMessage, NPopconfirm, NCard } from 'naive-ui';
import { fetchUserList, fetchCreateUser, fetchUpdateUser, fetchResetUserKey, fetchDeleteUser, fetchModelList } from '@/service/api';
import { fetchUserWhitelist, fetchAddWhitelist, fetchDeleteWhitelist } from '@/service/api/ip';
import type { DataTableColumns } from 'naive-ui';

const loading = ref(false);
const users = ref<Api.User.User[]>([]);
const dialog = useDialog();
const message = useMessage();
const isEdit = ref(false);
const editingId = ref<number | null>(null);

// IP Whitelist
const showWhitelistModal = ref(false);
const whitelistUserId = ref<number | null>(null);
const whitelistUserName = ref('');
const whitelistEntries = ref<Api.IpAccess.WhitelistEntry[]>([]);
const newWhitelistIp = ref('');

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
const formModel = ref({
  username: '',
  password: '',
  role: 'user',
  status: 'active',
  note: '',
  enabled_models: [] as string[]
});

const roleOptions = [
  { label: 'User', value: 'user' },
  { label: 'Admin', value: 'admin' }
];

const statusOptions = [
  { label: '活跃', value: 'active' },
  { label: '禁用', value: 'disabled' }
];

// Model options for MultiSelect
const modelOptions = ref<{ label: string; value: string }[]>([]);

async function loadModelOptions() {
  const { data } = await fetchModelList(1, 200);
  if (data) {
    const items = Array.isArray(data.items) ? data.items : (Array.isArray(data) ? data : []);
    modelOptions.value = items
      .filter((m: any) => m.enabled)
      .map((m: any) => ({ label: `${m.proxy_name} (${m.source_name})`, value: m.proxy_name }));
  }
}

// API Key mask: show first 8 chars, rest ****
function maskApiKey(key: string): string {
  if (!key || key.length <= 8) return key || '';
  return `${key.slice(0, 8)}****`;
}

function copyApiKey(key: string) {
  navigator.clipboard.writeText(key).then(() => {
    message.success('API Key 已复制到剪贴板');
  }).catch(() => {
    message.error('复制失败');
  });
}

// Whitelist management
async function openWhitelist(row: Api.User.User) {
  whitelistUserId.value = row.id;
  whitelistUserName.value = row.username;
  showWhitelistModal.value = true;
  await loadWhitelist(row.id);
}

async function loadWhitelist(userId: number) {
  const { data, error } = await fetchUserWhitelist(userId);
  if (!error && data) {
    whitelistEntries.value = data;
  }
}

async function handleAddWhitelist() {
  const ip = newWhitelistIp.value.trim();
  if (!ip || !whitelistUserId.value) {
    message.warning('请输入 IP 地址');
    return;
  }
  // Basic IP/CIDR validation
  const ipPattern = /^(\d{1,3}\.){3}\d{1,3}(\/\d{1,2})?$|^([0-9a-fA-F:]+)(\/\d{1,3})?$/;
  if (!ipPattern.test(ip)) {
    message.warning('IP 格式不正确，请输入 IPv4/IPv6 地址或 CIDR 网段');
    return;
  }
  const { error } = await fetchAddWhitelist({
    user_id: whitelistUserId.value,
    ip_address: ip
  });
  if (!error) {
    message.success('IP 已加入白名单');
    newWhitelistIp.value = '';
    if (whitelistUserId.value) await loadWhitelist(whitelistUserId.value);
  } else {
    message.error('添加失败');
  }
}

async function handleDeleteWhitelist(id: number) {
  const { error } = await fetchDeleteWhitelist(id);
  if (!error) {
    message.success('已从白名单移除');
    if (whitelistUserId.value) await loadWhitelist(whitelistUserId.value);
  } else {
    message.error('移除失败');
  }
}

// Calculate quota usage percentage
function getQuotaUsage(user: Api.User.User): { used: number; total: number; percent: number; label: string } | null {
  // Quota info would come from a separate API; for now show from user data if available
  if ((user as any).quota_used !== undefined && (user as any).quota_total !== undefined) {
    const used = (user as any).quota_used;
    const total = (user as any).quota_total;
    const percent = total > 0 ? Math.round((used / total) * 100) : 0;
    return { used, total, percent, label: `${used}/${total}` };
  }
  return null;
}

const columns: DataTableColumns<Api.User.User> = [
  { title: 'ID', key: 'id', width: 60 },
  { title: '用户名', key: 'username', width: 120 },
  {
    title: 'API Key',
    key: 'api_key',
    width: 180,
    render: row => h(NSpace, { align: 'center' }, {
      default: () => [
        h('span', { style: 'font-family: monospace; font-size: 12px' }, maskApiKey(row.api_key)),
        h(NButton, {
          size: 'tiny',
          text: true,
          onClick: () => copyApiKey(row.api_key)
        }, { default: () => '复制' })
      ]
    })
  },
  { title: '角色', key: 'role', width: 80, render: row => h(NTag, { type: row.role === 'admin' ? 'error' : 'info' }, { default: () => row.role }) },
  {
    title: '绑定模型',
    key: 'enabled_models',
    width: 150,
    render: row => {
      if (!row.enabled_models) return h(NTag, { type: 'default' }, { default: () => '全部' });
      try {
        const models: string[] = JSON.parse(row.enabled_models);
        return h(NSpace, { size: 4, wrap: true }, {
          default: () => models.slice(0, 2).map(m => h(NTag, { size: 'small', type: 'info' }, { default: () => m }))
            .concat(models.length > 2 ? [h(NTag, { size: 'small', type: 'default' }, { default: () => `+${models.length - 2}` })] : [])
        });
      } catch {
        return h('span', null, { default: () => '-' });
      }
    }
  },
  {
    title: '配额使用',
    key: 'quota_usage',
    width: 140,
    render: row => {
      const usage = getQuotaUsage(row);
      if (!usage) return h('span', { style: 'color: #999; font-size: 12px' }, '未设置');
      const color = usage.percent >= 90 ? '#e80c47' : usage.percent >= 70 ? '#f5a623' : '#18a058';
      return h('div', { style: 'width: 100%' }, [
        h('div', { style: 'font-size: 11px; margin-bottom: 2px; color: #666' }, usage.label),
        h(NProgress, {
          percentage: Math.min(usage.percent, 100),
          height: 6,
          color,
          showIndicator: false
        })
      ]);
    }
  },
  { title: '状态', key: 'status', width: 80, render: row => h(NTag, { type: row.status === 'active' ? 'success' : 'default' }, { default: () => row.status }) },
  {
    title: '操作',
    key: 'actions',
    width: 280,
    render: row => h(NSpace, {}, {
      default: () => [
        h(NButton, { size: 'small', onClick: () => handleEdit(row) }, { default: () => '编辑' }),
        h(NButton, { size: 'small', onClick: () => handleResetKey(row) }, { default: () => '重置Key' }),
        h(NButton, { size: 'small', type: 'warning', onClick: () => openWhitelist(row) }, { default: () => 'IP白名单' }),
        h(NPopconfirm, { onPositiveClick: () => handleDelete(row) }, {
          trigger: () => h(NButton, { size: 'small', type: 'error' }, { default: () => '删除' }),
          default: () => `确定要删除用户 "${row.username}" 吗？此操作不可撤销。`
        })
      ]
    })
  }
];

async function loadData() {
  loading.value = true;
  const { data, error } = await fetchUserList(pagination.value.page, pagination.value.pageSize);
  if (!error && data) {
    if (data.items && data.total !== undefined) {
      users.value = data.items;
      pagination.value.itemCount = data.total;
    } else {
      users.value = Array.isArray(data) ? data : [];
      pagination.value.itemCount = users.value.length;
    }
  }
  loading.value = false;
}

function handleCreate() {
  isEdit.value = false;
  editingId.value = null;
  formModel.value = { username: '', password: '', role: 'user', status: 'active', note: '', enabled_models: [] };
  showModal.value = true;
}

function handleEdit(row: Api.User.User) {
  isEdit.value = true;
  editingId.value = row.id;
  let enabledModels: string[] = [];
  if (row.enabled_models) {
    try {
      enabledModels = JSON.parse(row.enabled_models);
    } catch {
      message.warning('该用户的模型绑定配置格式异常，已重置为空');
    }
  }
  formModel.value = {
    username: row.username,
    password: '',
    role: row.role,
    status: row.status,
    note: row.note || '',
    enabled_models: enabledModels
  };
  showModal.value = true;
}

async function handleSave() {
  if (!formModel.value.username) {
    message.warning('请填写用户名');
    return;
  }

  let error: any;
  if (isEdit.value && editingId.value) {
    const payload: any = {
      username: formModel.value.username,
      role: formModel.value.role,
      status: formModel.value.status,
      note: formModel.value.note,
      enabled_models: formModel.value.enabled_models.length > 0 ? JSON.stringify(formModel.value.enabled_models) : null
    };
    const res = await fetchUpdateUser(editingId.value, payload);
    error = res.error;
  } else {
    if (!formModel.value.password) {
      message.warning('请填写密码');
      return;
    }
    const { data, error: err } = await fetchCreateUser({
      ...formModel.value,
      enabled_models: formModel.value.enabled_models.length > 0 ? JSON.stringify(formModel.value.enabled_models) : undefined
    });
    if (!err && data) {
      message.success(`用户创建成功！API Key: ${data.api_key}`);
      showModal.value = false;
      await loadData();
    }
    return;
  }

  if (!error) {
    message.success(isEdit.value ? '用户已更新' : '用户创建成功');
    showModal.value = false;
    await loadData();
  }
}

async function handleResetKey(row: Api.User.User) {
  dialog.warning({
    title: '确认重置',
    content: `确定要重置用户 "${row.username}" 的 API Key 吗？`,
    positiveText: '确定',
    negativeText: '取消',
    onPositiveClick: async () => {
      const { data, error } = await fetchResetUserKey(row.id);
      if (!error && data) {
        message.success(`Key 已重置！新 Key: ${data.api_key}`);
        await loadData();
      }
    }
  });
}

async function handleDelete(row: Api.User.User) {
  dialog.warning({
    title: '确认删除',
    content: `确定要删除用户 "${row.username}" 吗？此操作不可撤销。`,
    positiveText: '确定',
    negativeText: '取消',
    onPositiveClick: async () => {
      const { error } = await fetchDeleteUser(row.id);
      if (!error) {
        message.success('用户已删除');
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
          <span>用户管理</span>
          <NButton type="primary" @click="handleCreate">创建用户</NButton>
        </NSpace>
      </template>

      <NDataTable
        :columns="columns"
        :data="users"
        :loading="loading"
        :pagination="pagination"
        :remote="true"
        :row-key="(row: Api.User.User) => row.id"
      />
    </NCard>

    <NModal v-model:show="showModal" preset="card" :title="isEdit ? '编辑用户' : '创建用户'" style="width: 550px">
      <NForm :model="formModel" label-placement="left" label-width="80">
        <NFormItem label="用户名">
          <NInput v-model:value="formModel.username" placeholder="请输入用户名" />
        </NFormItem>
        <NFormItem v-if="!isEdit" label="密码">
          <NInput v-model:value="formModel.password" type="password" show-password-on="click" placeholder="请输入密码" />
        </NFormItem>
        <NFormItem label="角色">
          <NSelect v-model:value="formModel.role" :options="roleOptions" />
        </NFormItem>
        <NFormItem v-if="isEdit" label="状态">
          <NSelect v-model:value="formModel.status" :options="statusOptions" />
        </NFormItem>
        <NFormItem label="绑定模型">
          <NSelect
            v-model:value="formModel.enabled_models"
            multiple
            filterable
            :options="modelOptions"
            placeholder="不选则允许访问所有模型"
          />
        </NFormItem>
        <NFormItem label="备注">
          <NInput v-model:value="formModel.note" type="textarea" placeholder="可选" />
        </NFormItem>
      </NForm>
      <template #footer>
        <NSpace justify="end">
          <NButton @click="showModal = false">取消</NButton>
          <NButton type="primary" @click="handleSave">保存</NButton>
        </NSpace>
      </template>
    </NModal>

    <!-- IP Whitelist Modal -->
    <NModal v-model:show="showWhitelistModal" preset="card" :title="`IP 白名单 — ${whitelistUserName}`" style="width: 550px">
      <NSpace vertical :size="12">
        <NSpace>
          <NInput v-model:value="newWhitelistIp" placeholder="输入 IP 地址" style="width: 220px" @keyup.enter="handleAddWhitelist" />
          <NButton type="primary" @click="handleAddWhitelist">添加</NButton>
        </NSpace>
        <NSpace wrap>
          <NTag
            v-for="entry in whitelistEntries"
            :key="entry.id"
            closable
            type="info"
            @close="handleDeleteWhitelist(entry.id)"
          >
            {{ entry.ip_address }}
          </NTag>
          <span v-if="whitelistEntries.length === 0" style="color: #999; font-size: 12px">暂无白名单 IP</span>
        </NSpace>
      </NSpace>
    </NModal>
  </NSpace>
</template>

<style scoped></style>
