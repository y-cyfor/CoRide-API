<script setup lang="ts">
import { ref, h, computed, onMounted } from 'vue';
import {
  NButton, NTag, NSpace, NForm, NFormItem, NSelect,
  NInputNumber, NCard, NDataTable, useDialog, useMessage, NPopconfirm
} from 'naive-ui';
import type { DataTableColumns } from 'naive-ui';
import {
  fetchAppProfileList,
  fetchChannelList,
  fetchGlobalTrafficPlan,
  fetchUpsertGlobalTrafficPlan,
  fetchDeleteGlobalTrafficPlan,
  fetchChannelTrafficPlans,
  fetchUpsertChannelTrafficPlan,
  fetchDeleteChannelTrafficPlan
} from '@/service/api';

interface EditingTimeSlot {
  id: number; // negative for new slots
  start_hour: number;
  end_hour: number;
  apps: Array<{ app_profile_id: number; weight: number }>;
}

const message = useMessage();
const dialog = useDialog();

const loading = ref(false);
const profileOptions = ref<{ label: string; value: number }[]>([]);
const channelOptions = ref<{ label: string; value: number }[]>([]);
const channelMap = ref<Map<number, string>>(new Map());

// Global plan inline editing
const editingSlots = ref<EditingTimeSlot[]>([]);
const isEditingGlobal = ref(false);

// Channel plans
const globalPlan = ref<Api.TrafficPlan.PlanDetail | null>(null);
const channelPlans = ref<Api.TrafficPlan.PlanDetail[]>([]);

// Channel plan editing modal
const showChannelModal = ref(false);
const editingChannelId = ref<number | null>(null);
const channelSlotData = ref<EditingTimeSlot[]>([]);

// Hour options 0-24
const hourOptions = Array.from({ length: 25 }, (_, i) => ({
  label: `${i.toString().padStart(2, '0')}:00`,
  value: i
}));

// Available app options for selector (exclude already selected apps in this slot)
function getAvailableApps(excludeId: number): Array<{ label: string; value: number }> {
  return profileOptions.value;
}

// Calculate weight sum for a slot
function slotWeightSum(slot: EditingTimeSlot): number {
  return slot.apps.reduce((sum, a) => sum + a.weight, 0);
}

// Check time range overlap
function hasOverlap(slots: EditingTimeSlot[], excludeId: number, start: number, end: number): boolean {
  return slots.some(s => s.id !== excludeId && s.start_hour < end && s.end_hour > start);
}

// Time slot table columns (global)
const timeSlotColumns: DataTableColumns<Api.TrafficPlan.Slot> = [
  { title: '时段', key: 'time_range', width: 160, render: row => `${row.start_hour}:00 - ${row.end_hour}:00` },
  { title: '应用', key: 'app_profile_name', width: 140 },
  { title: '比例', key: 'weight', width: 80, render: row => `${row.weight}%` },
];

// Channel plan columns
const channelPlanColumns: DataTableColumns<Api.TrafficPlan.PlanDetail> = [
  { title: '渠道', key: 'channel_name', width: 140, render: row => channelMap.value.get(row.channel_id!) || `渠道#${row.channel_id}` },
  { title: '时段数', key: 'slot_count', width: 80, render: row => row.slots.length },
  {
    title: '操作', key: 'actions', width: 160, render: row => h(NSpace, {}, {
      default: () => [
        h(NButton, { size: 'small', onClick: () => editChannelPlan(row.channel_id!) }, { default: () => '编辑' }),
        h(NPopconfirm, { onPositiveClick: () => deleteChannelPlan(row.channel_id!) }, {
          trigger: () => h(NButton, { size: 'small', type: 'error' }, { default: () => '删除' }),
          default: () => `确定要删除该渠道的方案吗？`
        })
      ]
    })
  }
];

// Channel editing slot columns
const channelTimeSlotColumns: DataTableColumns<Api.TrafficPlan.Slot> = [
  { title: '时段', key: 'time_range', width: 160, render: row => `${row.start_hour}:00 - ${row.end_hour}:00` },
  { title: '应用', key: 'app_profile_name', width: 140 },
  { title: '比例', key: 'weight', width: 80, render: row => `${row.weight}%` },
];

// Load data
async function loadData() {
  loading.value = true;
  try {
    const { data: pData } = await fetchAppProfileList(1, 100);
    if (pData) {
      const pList = (pData as any).items || (Array.isArray(pData) ? pData : []);
      profileOptions.value = pList.map((p: any) => ({ label: p.name, value: p.id }));
    }

    const { data: chData } = await fetchChannelList(1, 100);
    if (chData) {
      const items = (chData as any).items || chData;
      const list = Array.isArray(items) ? items : [];
      channelOptions.value = list.map((c: any) => ({ label: c.name, value: c.id }));
      channelMap.value = new Map(list.map((c: any) => [c.id, c.name]));
    }

    const { data: gData } = await fetchGlobalTrafficPlan();
    globalPlan.value = gData;

    const { data: cpData } = await fetchChannelTrafficPlans();
    channelPlans.value = cpData || [];
  } catch {
    message.error('加载数据失败');
  }
  loading.value = false;
}

// Global plan inline editing
function startEditGlobal() {
  if (globalPlan.value) {
    // Group slots by time range
    const groups = new Map<string, EditingTimeSlot[]>();
    for (const s of globalPlan.value.slots) {
      const key = `${s.start_hour}-${s.end_hour}`;
      if (!groups.has(key)) groups.set(key, []);
      groups.get(key)!.push({ id: s.id, start_hour: s.start_hour, end_hour: s.end_hour, apps: [{ app_profile_id: s.app_profile_id, weight: s.weight }] });
    }

    editingSlots.value = Array.from(groups.entries()).map(([key, slots], idx) => ({
      id: idx + 1,
      start_hour: slots[0].start_hour,
      end_hour: slots[0].end_hour,
      apps: slots.map(s => s.apps[0])
    }));
  } else {
    editingSlots.value = [];
  }
  isEditingGlobal.value = true;
}

function cancelEditGlobal() {
  isEditingGlobal.value = false;
  editingSlots.value = [];
}

function addTimeSlot() {
  const maxId = editingSlots.value.reduce((max, s) => Math.max(max, s.id), 0);
  editingSlots.value.push({ id: -(maxId + 1), start_hour: 0, end_hour: 24, apps: [] });
}

function removeTimeSlot(idx: number) {
  editingSlots.value.splice(idx, 1);
}

function addAppToSlot(slot: EditingTimeSlot) {
  const usedIds = new Set(slot.apps.map(a => a.app_profile_id));
  const available = profileOptions.value.filter(p => !usedIds.has(p.value));
  if (available.length === 0) {
    message.warning('没有更多可选应用');
    return;
  }
  const remaining = 100 - slotWeightSum(slot);
  slot.apps.push({ app_profile_id: available[0].value, weight: remaining > 0 ? remaining : 10 });
}

function removeAppFromSlot(slot: EditingTimeSlot, appIdx: number) {
  slot.apps.splice(appIdx, 1);
}

async function saveGlobalPlan() {
  if (editingSlots.value.length === 0) {
    message.warning('请至少添加一个时段');
    return;
  }

  // Validate each slot
  for (let i = 0; i < editingSlots.value.length; i++) {
    const slot = editingSlots.value[i];
    if (slot.start_hour >= slot.end_hour) {
      message.warning(`时段${i + 1} 的结束时间必须大于开始时间`);
      return;
    }
    if (slot.apps.length === 0) {
      message.warning(`时段${i + 1} 请至少添加一个应用`);
      return;
    }
    const weightSum = slotWeightSum(slot);
    if (weightSum !== 100) {
      message.warning(`时段${i + 1} 的应用比例之和必须为 100%，当前为 ${weightSum}%`);
      return;
    }
    for (const app of slot.apps) {
      if (!app.app_profile_id) {
        message.warning(`时段${i + 1} 中存在未选择的应用`);
        return;
      }
    }
  }

  // Check overlap
  for (let i = 0; i < editingSlots.value.length; i++) {
    for (let j = i + 1; j < editingSlots.value.length; j++) {
      const a = editingSlots.value[i];
      const b = editingSlots.value[j];
      if (a.start_hour < b.end_hour && a.end_hour > b.start_hour) {
        message.warning(`时段 ${a.start_hour}:00-${a.end_hour}:00 与 ${b.start_hour}:00-${b.end_hour}:00 重叠，请调整`);
        return;
      }
    }
  }

  // Flatten to API format
  const slots: Api.TrafficPlan.SlotRequest[] = [];
  for (const ts of editingSlots.value) {
    for (const app of ts.apps) {
      slots.push({
        start_hour: ts.start_hour,
        end_hour: ts.end_hour,
        app_profile_id: app.app_profile_id,
        weight: app.weight
      });
    }
  }

  const { error } = await fetchUpsertGlobalTrafficPlan({ slots });
  if (!error) {
    message.success('全局方案已保存');
    isEditingGlobal.value = false;
    await loadData();
  }
}

async function deleteGlobalPlan() {
  dialog.warning({
    title: '确认删除',
    content: '确定要删除全局方案吗？',
    positiveText: '确定',
    negativeText: '取消',
    onPositiveClick: async () => {
      const { error } = await fetchDeleteGlobalTrafficPlan();
      if (!error) {
        message.success('全局方案已删除');
        await loadData();
      }
    }
  });
}

// Channel plan editing
function editChannelPlan(channelId: number | null) {
  editingChannelId.value = channelId;
  if (channelId) {
    const plan = channelPlans.value.find(p => p.channel_id === channelId);
    if (plan) {
      const groups = new Map<string, EditingTimeSlot[]>();
      for (const s of plan.slots) {
        const key = `${s.start_hour}-${s.end_hour}`;
        if (!groups.has(key)) groups.set(key, []);
        groups.get(key)!.push({ id: s.id, start_hour: s.start_hour, end_hour: s.end_hour, apps: [{ app_profile_id: s.app_profile_id, weight: s.weight }] });
      }
      channelSlotData.value = Array.from(groups.entries()).map(([key, slots], idx) => ({
        id: idx + 1,
        start_hour: slots[0].start_hour,
        end_hour: slots[0].end_hour,
        apps: slots.map(s => s.apps[0])
      }));
    }
  } else {
    channelSlotData.value = [];
  }
  showChannelModal.value = true;
}

function addChannelTimeSlot() {
  const maxId = channelSlotData.value.reduce((max, s) => Math.max(max, s.id), 0);
  channelSlotData.value.push({ id: -(maxId + 1), start_hour: 0, end_hour: 24, apps: [] });
}

function removeChannelTimeSlot(idx: number) {
  channelSlotData.value.splice(idx, 1);
}

function addAppToChannelSlot(slot: EditingTimeSlot) {
  const remaining = 100 - slotWeightSum(slot);
  slot.apps.push({ app_profile_id: profileOptions.value[0]?.value || 0, weight: remaining > 0 ? remaining : 10 });
}

function removeAppFromChannelSlot(slot: EditingTimeSlot, appIdx: number) {
  slot.apps.splice(appIdx, 1);
}

async function saveChannelPlan() {
  if (!editingChannelId.value) {
    message.warning('请选择要配置方案的渠道');
    return;
  }
  if (channelSlotData.value.length === 0) {
    message.warning('请至少添加一个时段');
    return;
  }

  for (let i = 0; i < channelSlotData.value.length; i++) {
    const slot = channelSlotData.value[i];
    if (slot.start_hour >= slot.end_hour) {
      message.warning(`时段${i + 1} 的结束时间必须大于开始时间`);
      return;
    }
    if (slot.apps.length === 0) {
      message.warning(`时段${i + 1} 请至少添加一个应用`);
      return;
    }
    const weightSum = slotWeightSum(slot);
    if (weightSum !== 100) {
      message.warning(`时段${i + 1} 的应用比例之和必须为 100%，当前为 ${weightSum}%`);
      return;
    }
  }

  for (let i = 0; i < channelSlotData.value.length; i++) {
    for (let j = i + 1; j < channelSlotData.value.length; j++) {
      const a = channelSlotData.value[i];
      const b = channelSlotData.value[j];
      if (a.start_hour < b.end_hour && a.end_hour > b.start_hour) {
        message.warning(`时段 ${a.start_hour}:00-${a.end_hour}:00 与 ${b.start_hour}:00-${b.end_hour}:00 重叠，请调整`);
        return;
      }
    }
  }

  const slots: Api.TrafficPlan.SlotRequest[] = [];
  for (const ts of channelSlotData.value) {
    for (const app of ts.apps) {
      slots.push({ start_hour: ts.start_hour, end_hour: ts.end_hour, app_profile_id: app.app_profile_id, weight: app.weight });
    }
  }

  const { error } = await fetchUpsertChannelTrafficPlan(editingChannelId.value, { slots });
  if (!error) {
    message.success('渠道方案已保存');
    showChannelModal.value = false;
    await loadData();
  }
}

async function deleteChannelPlan(channelId: number) {
  const { error } = await fetchDeleteChannelTrafficPlan(channelId);
  if (!error) {
    message.success('渠道方案已删除');
    await loadData();
  }
}

onMounted(() => {
  loadData();
});
</script>

<template>
  <NSpace vertical :size="16">
    <!-- Global Plan -->
    <NCard title="全局方案" :bordered="false" class="card-wrapper">
      <template #header-extra>
        <NSpace v-if="!isEditingGlobal">
          <NButton size="small" type="primary" @click="startEditGlobal">
            {{ globalPlan ? '编辑方案' : '设置方案' }}
          </NButton>
          <NButton v-if="globalPlan" size="small" type="error" @click="deleteGlobalPlan">删除方案</NButton>
        </NSpace>
        <NSpace v-else>
          <NButton size="small" @click="cancelEditGlobal">取消</NButton>
          <NButton size="small" type="primary" @click="saveGlobalPlan">保存方案</NButton>
        </NSpace>
      </template>

      <!-- Display mode -->
      <div v-if="!isEditingGlobal && globalPlan && globalPlan.slots.length > 0">
        <NDataTable
          :columns="timeSlotColumns"
          :data="globalPlan.slots"
          :row-key="(row: Api.TrafficPlan.Slot) => row.id"
          :pagination="false"
        />
      </div>
      <div v-else-if="!isEditingGlobal" style="color: #999; text-align: center; padding: 24px;">
        尚未配置全局方案，所有未单独设置的渠道将不使用应用伪装
      </div>

      <!-- Edit mode: each time slot has its own app list -->
      <div v-else style="width: 100%">
        <NSpace vertical :size="12">
          <div
            v-for="(slot, sIdx) in editingSlots"
            :key="slot.id"
            style="border: 1px solid #e0e0e0; border-radius: 8px; padding: 12px;"
          >
            <NSpace align="center" style="margin-bottom: 8px;">
              <span style="font-weight: 600; min-width: 60px;">时段{{ sIdx + 1 }}</span>
              <NSelect v-model:value="slot.start_hour" :options="hourOptions" style="width: 110px" placeholder="开始" />
              <span>至</span>
              <NSelect v-model:value="slot.end_hour" :options="hourOptions" style="width: 110px" placeholder="结束" />
              <NButton size="tiny" type="error" text @click="removeTimeSlot(sIdx)">删除时段</NButton>
            </NSpace>

            <!-- Apps for this time slot -->
            <div style="margin-bottom: 8px;">
              <div style="display: flex; gap: 12px; align-items: center; margin-bottom: 4px; padding: 0 8px;">
                <span style="font-size: 12px; color: #999; flex: 1;">应用</span>
                <span style="font-size: 12px; color: #999; width: 120px;">比例(%)</span>
                <span style="width: 50px;"></span>
              </div>
              <div v-for="(app, aIdx) in slot.apps" :key="aIdx" style="display: flex; gap: 12px; align-items: center; margin-bottom: 4px; padding: 0 8px;">
                <NSelect v-model:value="app.app_profile_id" :options="profileOptions" style="width: 220px;" placeholder="选择应用" />
                <NInputNumber v-model:value="app.weight" :min="1" :max="100" :style="{ width: '120px' }" placeholder="比例" />
                <NButton size="tiny" type="error" text @click="removeAppFromSlot(slot, aIdx)">删除</NButton>
              </div>
            </div>
            <NSpace>
              <NButton size="tiny" @click="addAppToSlot(slot)">+ 添加应用</NButton>
              <span v-if="slot.apps.length > 0" style="font-size: 12px; color: #666;">
                合计: {{ slotWeightSum(slot) }}%
                <span v-if="slotWeightSum(slot) !== 100" style="color: #e80c47;">（必须为 100%）</span>
                <span v-else style="color: #18a058;">✓</span>
              </span>
            </NSpace>
          </div>
          <NButton size="small" @click="addTimeSlot">+ 添加时段</NButton>
        </NSpace>
      </div>
    </NCard>

    <!-- Channel Plans -->
    <NCard title="渠道方案" :bordered="false" class="card-wrapper">
      <template #header-extra>
        <NButton size="small" type="primary" @click="editChannelPlan(null)">新增渠道方案</NButton>
      </template>

      <NDataTable
        v-if="channelPlans.length > 0"
        :columns="channelPlanColumns"
        :data="channelPlans"
        :row-key="(row: Api.TrafficPlan.PlanDetail) => row.id"
        :pagination="false"
      />
      <div v-else style="color: #999; text-align: center; padding: 24px;">
        尚未配置渠道专属方案
      </div>
    </NCard>

    <!-- Channel Plan Edit Modal -->
    <NModal v-model:show="showChannelModal" preset="card" title="编辑渠道方案" style="width: 800px">
      <NForm>
        <NFormItem v-if="!editingChannelId" label="渠道">
          <NSelect v-model:value="editingChannelId" :options="channelOptions" placeholder="选择要配置方案的渠道" style="width: 300px" />
        </NFormItem>

        <NSpace vertical :size="12">
          <div
            v-for="(slot, sIdx) in channelSlotData"
            :key="slot.id"
            style="border: 1px solid #e0e0e0; border-radius: 8px; padding: 12px;"
          >
            <NSpace align="center" style="margin-bottom: 8px;">
              <span style="font-weight: 600; min-width: 60px;">时段{{ sIdx + 1 }}</span>
              <NSelect v-model:value="slot.start_hour" :options="hourOptions" style="width: 110px" placeholder="开始" />
              <span>至</span>
              <NSelect v-model:value="slot.end_hour" :options="hourOptions" style="width: 110px" placeholder="结束" />
              <NButton size="tiny" type="error" text @click="removeChannelTimeSlot(sIdx)">删除时段</NButton>
            </NSpace>

            <div style="margin-bottom: 8px;">
              <div style="display: flex; gap: 12px; align-items: center; margin-bottom: 4px; padding: 0 8px;">
                <span style="font-size: 12px; color: #999; flex: 1;">应用</span>
                <span style="font-size: 12px; color: #999; width: 120px;">比例(%)</span>
                <span style="width: 50px;"></span>
              </div>
              <div v-for="(app, aIdx) in slot.apps" :key="aIdx" style="display: flex; gap: 12px; align-items: center; margin-bottom: 4px; padding: 0 8px;">
                <NSelect v-model:value="app.app_profile_id" :options="profileOptions" style="width: 220px;" placeholder="选择应用" />
                <NInputNumber v-model:value="app.weight" :min="1" :max="100" :style="{ width: '120px' }" placeholder="比例" />
                <NButton size="tiny" type="error" text @click="removeAppFromChannelSlot(slot, aIdx)">删除</NButton>
              </div>
            </div>
            <NSpace>
              <NButton size="tiny" @click="addAppToChannelSlot(slot)">+ 添加应用</NButton>
              <span v-if="slot.apps.length > 0" style="font-size: 12px; color: #666;">
                合计: {{ slotWeightSum(slot) }}%
                <span v-if="slotWeightSum(slot) !== 100" style="color: #e80c47;">（必须为 100%）</span>
                <span v-else style="color: #18a058;">✓</span>
              </span>
            </NSpace>
          </div>
          <NButton size="small" @click="addChannelTimeSlot">+ 添加时段</NButton>
        </NSpace>
      </NForm>
      <template #footer>
        <NSpace justify="end">
          <NButton @click="showChannelModal = false">取消</NButton>
          <NButton type="primary" @click="saveChannelPlan">保存</NButton>
        </NSpace>
      </template>
    </NModal>
  </NSpace>
</template>

<style scoped></style>
