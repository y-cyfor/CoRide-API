<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, nextTick, h } from 'vue';
import { useAppStore } from '@/store/modules/app';
import { useAuthStore } from '@/store/modules/auth';
import { NCard, NSpace, NStatistic, NTag, NSelect, NButton, NGrid, NGi, NInputNumber, NDataTable, NAlert } from 'naive-ui';
import { fetchUsageStats, fetchDashboardStats, fetchChannelList, fetchModelList, fetchRecentLogs, fetchQuotaWarnings } from '@/service/api';
import { request } from '@/service/request';
import * as echarts from 'echarts';
import type { EChartsOption } from 'echarts';

const appStore = useAppStore();
const authStore = useAuthStore();
const gap = computed(() => (appStore.isMobile ? 0 : 16));
const isAdmin = computed(() => authStore.userInfo.role === 'admin');

const loading = ref(false);
const stats = ref({
  total_requests: 0,
  today_requests: 0,
  active_users: 0,
  total_tokens: 0,
  p95_latency_ms: 0,
  error_rate: '0.0%'
});
const usageData = ref<any>(null);
const recentLogs = ref<Api.Log.RequestLog[]>([]);
const quotaWarnings = ref<Array<{
  user_id: number;
  username: string;
  quota_type: string;
  total_limit: number;
  used: number;
  percent: number;
  severity: 'info' | 'warning' | 'critical';
  message: string;
}>>([]);

const trendChartRef = ref<HTMLDivElement>();
const channelChartRef = ref<HTMLDivElement>();
const tokenChartRef = ref<HTMLDivElement>();
let trendChart: echarts.ECharts | null = null;
let channelChart: echarts.ECharts | null = null;
let tokenChart: echarts.ECharts | null = null;

// Filter state
const filterChannel = ref<number | undefined>(undefined);
const filterModel = ref<string>('');
const filterDays = ref(7);
const channelOptions = ref<{ label: string; value: number }[]>([]);
const modelOptions = ref<{ label: string; value: string }[]>([]);

async function loadFilterOptions() {
  if (!isAdmin.value) return;
  const { data: chData } = await fetchChannelList(1, 100);
  if (chData) {
    const items = chData.items || chData;
    channelOptions.value = [
      { label: '全部渠道', value: -1 as any },
      ...(Array.isArray(items) ? items : []).map((c: any) => ({ label: c.name, value: c.id }))
    ];
  }
  const { data: mData } = await fetchModelList(1, 200);
  if (mData) {
    const items = mData.items || mData;
    modelOptions.value = [
      { label: '全部模型', value: '' },
      ...(Array.isArray(items) ? items : []).map((m: any) => ({ label: m.proxy_name, value: m.proxy_name }))
    ];
  }
}

async function loadData() {
  loading.value = true;

  if (isAdmin.value) {
    const [dashRes, usageRes] = await Promise.all([
      fetchDashboardStats(),
      fetchUsageStats({
        channel_id: filterChannel.value,
        model: filterModel.value || undefined,
        days: filterDays.value
      })
    ]);
    if (dashRes.data) {
      Object.assign(stats.value, dashRes.data);
      stats.value.total_tokens = usageRes.data?.total_tokens || 0;
    }
    if (usageRes.data) {
      usageData.value = usageRes.data;
      await nextTick();
      renderTrendChart(usageRes.data.daily_trend);
      renderChannelChart(usageRes.data.channel_usage);
      renderTokenChart(usageRes.data.token_daily || []);
    }
    const { data: recent, error: err3 } = await fetchRecentLogs(10);
    if (!err3 && recent) recentLogs.value = recent;
    const { data: warnings, error: err4 } = await fetchQuotaWarnings();
    if (!err4 && warnings) quotaWarnings.value = warnings.warnings || [];
  } else {
    const [dashRes, usageRes] = await Promise.all([
      request({ url: '/user/stats/dashboard', method: 'get' }),
      request({ url: '/user/stats/usage', method: 'get', params: { days: filterDays.value } })
    ]);
    if (dashRes.data) Object.assign(stats.value, dashRes.data);
    if (usageRes.data) {
      usageData.value = usageRes.data;
      await nextTick();
      renderTrendChart(usageRes.data.daily_trend);
      renderTokenChart(usageRes.data.token_daily || []);
    }
  }

  loading.value = false;
}

function handleFilter() {
  loadData();
}

function handleReset() {
  filterChannel.value = undefined;
  filterModel.value = '';
  filterDays.value = 7;
  loadData();
}

function renderTrendChart(data: Array<{ day: string; count: number }>) {
  if (!trendChartRef.value) return;
  if (!trendChart) trendChart = echarts.init(trendChartRef.value);

  const option: EChartsOption = {
    title: { text: '', left: 'center', textStyle: { fontSize: 0 } },
    tooltip: { trigger: 'axis' },
    grid: { left: '3%', right: '4%', bottom: '3%', containLabel: true },
    xAxis: {
      type: 'category',
      data: data.map(d => d.day),
      boundaryGap: false
    },
    yAxis: { type: 'value' },
    series: [{
      name: '请求数',
      type: 'line',
      smooth: true,
      areaStyle: { opacity: 0.3 },
      data: data.map(d => d.count),
      itemStyle: { color: '#18a058' }
    }]
  };

  trendChart.setOption(option);
}

function renderChannelChart(data: Array<{ name: string; count: number }>) {
  if (!channelChartRef.value) return;
  if (!channelChart) channelChart = echarts.init(channelChartRef.value);

  const option: EChartsOption = {
    title: { text: '', left: 'center' },
    tooltip: { trigger: 'item', formatter: '{b}: {c}次 ({d}%)' },
    legend: { bottom: '5%' },
    series: [{
      type: 'pie',
      radius: ['40%', '70%'],
      avoidLabelOverlap: false,
      itemStyle: { borderRadius: 10, borderColor: '#fff', borderWidth: 2 },
      label: { show: false, position: 'center' },
      emphasis: { label: { show: true, fontSize: 18, fontWeight: 'bold' } },
      data: data.map(d => ({ name: d.name, value: d.count }))
    }]
  };

  channelChart.setOption(option);
}

function renderTokenChart(data: Array<{ day: string; prompt_tokens: number; completion_tokens: number }>) {
  if (!tokenChartRef.value) return;
  if (!tokenChart) tokenChart = echarts.init(tokenChartRef.value);

  const option: EChartsOption = {
    title: { text: '', left: 'center' },
    tooltip: { trigger: 'axis', axisPointer: { type: 'shadow' } },
    legend: { bottom: 0 },
    grid: { left: '3%', right: '4%', bottom: '10%', containLabel: true },
    xAxis: { type: 'category', data: data.map(d => d.day) },
    yAxis: { type: 'value', name: 'Tokens' },
    series: [
      {
        name: '输入 Tokens',
        type: 'bar',
        stack: 'token',
        data: data.map(d => d.prompt_tokens),
        itemStyle: { color: '#2080f0' }
      },
      {
        name: '输出 Tokens',
        type: 'bar',
        stack: 'token',
        data: data.map(d => d.completion_tokens),
        itemStyle: { color: '#18a058' }
      }
    ]
  };

  tokenChart.setOption(option);
}

function handleResize() {
  trendChart?.resize();
  channelChart?.resize();
  tokenChart?.resize();
}

onMounted(() => {
  loadFilterOptions();
  loadData();
  window.addEventListener('resize', handleResize);
});

onUnmounted(() => {
  window.removeEventListener('resize', handleResize);
  trendChart?.dispose();
  channelChart?.dispose();
  tokenChart?.dispose();
});
</script>

<template>
  <NSpace vertical :size="16">
    <!-- Quota warning alerts -->
    <NAlert
      v-for="warning in quotaWarnings"
      :key="`${warning.user_id}-${warning.quota_type}`"
      :type="warning.severity === 'critical' ? 'error' : warning.severity === 'warning' ? 'warning' : 'info'"
      :title="`配额预警: ${warning.username}`"
      closable
    >
      {{ warning.message }}
    </NAlert>

    <!-- Filter bar -->
    <NCard :bordered="false" class="card-wrapper">
      <NGrid :x-gap="12" :y-gap="8" responsive="screen" item-responsive>
        <NGi span="24 s:8 m:5">
          <NSelect
            v-model:value="filterChannel"
            :options="channelOptions"
            placeholder="按渠道筛选"
            clearable
          />
        </NGi>
        <NGi span="24 s:8 m:5">
          <NSelect
            v-model:value="filterModel"
            :options="modelOptions"
            placeholder="按模型筛选"
            clearable
            filterable
          />
        </NGi>
        <NGi span="24 s:8 m:4">
          <NInputNumber v-model:value="filterDays" :min="1" :max="90" placeholder="天数" />
        </NGi>
        <NGi span="24 s:24 m:10">
          <NSpace>
            <NButton type="primary" @click="handleFilter">筛选</NButton>
            <NButton @click="handleReset">重置</NButton>
          </NSpace>
        </NGi>
      </NGrid>
    </NCard>

    <!-- Stats Cards: uniform grid layout -->
    <NGrid :x-gap="16" :y-gap="16" responsive="screen" cols="1 s:2 m:5" item-responsive>
      <NGi>
        <NCard :bordered="false" class="card-wrapper">
          <NStatistic label="总请求数" :value="stats.total_requests" />
        </NCard>
      </NGi>
      <NGi>
        <NCard :bordered="false" class="card-wrapper">
          <NStatistic label="今日请求" :value="stats.today_requests" />
        </NCard>
      </NGi>
      <NGi>
        <NCard :bordered="false" class="card-wrapper">
          <NStatistic label="活跃用户" :value="stats.active_users" />
        </NCard>
      </NGi>
      <NGi>
        <NCard :bordered="false" class="card-wrapper">
          <NStatistic label="P95 延迟">
            <template #default>{{ stats.p95_latency_ms }}</template>
            <template #suffix>ms</template>
          </NStatistic>
        </NCard>
      </NGi>
      <NGi>
        <NCard :bordered="false" class="card-wrapper">
          <NStatistic label="错误率" :value="stats.error_rate" />
        </NCard>
      </NGi>
    </NGrid>

    <!-- Charts -->
    <NGrid :x-gap="gap" :y-gap="16" responsive="screen" item-responsive>
      <NGi span="24 s:24 m:14">
        <NCard :bordered="false" class="card-wrapper">
          <template #header><span class="card-title">近7天请求趋势</span></template>
          <div ref="trendChartRef" style="width: 100%; height: 350px"></div>
        </NCard>
      </NGi>
      <NGi span="24 s:24 m:10">
        <NCard :bordered="false" class="card-wrapper">
          <template #header><span class="card-title">渠道使用分布</span></template>
          <div ref="channelChartRef" style="width: 100%; height: 350px"></div>
        </NCard>
      </NGi>
    </NGrid>

    <!-- Token consumption chart -->
    <NCard :bordered="false" class="card-wrapper">
      <template #header><span class="card-title">近7天 Token 消耗（输入/输出）</span></template>
      <div ref="tokenChartRef" style="width: 100%; height: 350px"></div>
    </NCard>

    <!-- Recent request -->
    <NCard v-if="recentLogs.length" :bordered="false" class="card-wrapper">
      <template #header><span class="card-title">最近请求</span></template>
      <NDataTable
        :columns="[
          { title: 'ID', key: 'id', width: 60 },
          { title: '模型', key: 'model', width: 120 },
          { title: '端点', key: 'endpoint', width: 120 },
          { title: '状态', key: 'status_code', width: 80, render: (row: any) => {
            const type = row.status_code < 300 ? 'success' : row.status_code < 400 ? 'warning' : 'error';
            return h(NTag, { type, size: 'small' }, { default: () => row.status_code });
          }},
          { title: 'Tokens', key: 'total_tokens', width: 80 },
          { title: '耗时', key: 'elapsed_ms', width: 80, render: (row: any) => `${row.elapsed_ms}ms` },
          { title: '时间', key: 'created_at', width: 180, render: (row: any) => new Date(row.created_at).toLocaleString() },
          { title: '错误', key: 'error_message', ellipsis: { tooltip: true }, render: (row: any) => row.error_message || '-' }
        ]"
        :data="recentLogs"
        :pagination="false"
        :max-height="300"
      />
    </NCard>
  </NSpace>
</template>

<style scoped>
.card-title {
  font-size: 15px;
  font-weight: 600;
  color: var(--n-text-color);
}
</style>
