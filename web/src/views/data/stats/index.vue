<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { useAuthStore } from '@/store/modules/auth';
import { NCard, NSpace, NStatistic, NTag, NSelect, NButton, NGrid, NGi, NInputNumber } from 'naive-ui';
import { fetchUsageStats, fetchDashboardStats, fetchChannelList, fetchModelList } from '@/service/api';
import { request } from '@/service/request';
import * as echarts from 'echarts';
import type { EChartsOption } from 'echarts';

const authStore = useAuthStore();
const isAdmin = ref(authStore.userInfo.role === 'admin');

const loading = ref(false);
const stats = ref({
  total_requests: 0,
  today_requests: 0,
  active_users: 0,
  success_count: 0,
  failure_count: 0,
  total_tokens: 0,
  p95_latency_ms: 0,
  error_rate: '0.0%'
});
const trendChartRef = ref<HTMLDivElement>();
const channelChartRef = ref<HTMLDivElement>();
const tokenChartRef = ref<HTMLDivElement>();
const userRankChartRef = ref<HTMLDivElement>();
const modelChartRef = ref<HTMLDivElement>();
let trendChart: echarts.ECharts | null = null;
let channelChart: echarts.ECharts | null = null;
let tokenChart: echarts.ECharts | null = null;
let userRankChart: echarts.ECharts | null = null;
let modelChart: echarts.ECharts | null = null;

// Filter state
const filterChannel = ref<number | undefined>(undefined);
const filterModel = ref<string>('');
const filterDays = ref(7);
const channelOptions = ref<{ label: string; value: number }[]>([]);
const modelOptions = ref<{ label: string; value: string }[]>([]);

async function loadFilterOptions() {
  const { data: chData } = await fetchChannelList(1, 100);
  if (chData) {
    const items = chData.items || chData;
    channelOptions.value = [
      { label: '全部渠道', value: -1 as any },
      ...(Array.isArray(items) ? items : []).map((c: any) => ({ label: c.name, value: c.id }))
    ];
  }
  const { data: mData } = await fetchModelList(1, 1000);
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

  let usageRes: any;
  if (isAdmin.value) {
    const [dashRes, uRes] = await Promise.all([
      fetchDashboardStats(),
      fetchUsageStats({
        channel_id: filterChannel.value,
        model: filterModel.value || undefined,
        days: filterDays.value
      })
    ]);
    if (dashRes.data) {
      stats.value = {
        total_requests: dashRes.data.total_requests || 0,
        today_requests: dashRes.data.today_requests || 0,
        active_users: dashRes.data.active_users || 0,
        success_count: dashRes.data.success_count || 0,
        failure_count: dashRes.data.failure_count || 0,
        total_tokens: uRes.data?.total_tokens || 0,
        p95_latency_ms: dashRes.data.p95_latency_ms || 0,
        error_rate: dashRes.data.error_rate || '0.0%'
      };
    }
    usageRes = uRes;
  } else {
    const [dashRes, uRes] = await Promise.all([
      request({ url: '/user/stats/dashboard', method: 'get' }),
      request({ url: '/user/stats/usage', method: 'get', params: { days: filterDays.value } })
    ]);
    if (dashRes.data) {
      stats.value = {
        total_requests: dashRes.data.total_requests || 0,
        today_requests: dashRes.data.today_requests || 0,
        active_users: dashRes.data.active_users || 0,
        success_count: dashRes.data.success_count || 0,
        failure_count: dashRes.data.failure_count || 0,
        total_tokens: uRes.data?.total_tokens || 0,
        p95_latency_ms: dashRes.data.p95_latency_ms || 0,
        error_rate: dashRes.data.error_rate || '0.0%'
      };
    }
    usageRes = uRes;
  }

  if (usageRes.data) {
    renderTrendChart(usageRes.data.daily_trend || []);
    renderChannelChart(usageRes.data.channel_usage || []);
    renderTokenChart(usageRes.data.token_daily || []);
    renderUserRankChart(usageRes.data.top_users || []);
    renderModelChart(usageRes.data.model_usage || []);
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
  trendChart.setOption({
    title: { text: '请求趋势', left: 'center', textStyle: { fontSize: 14 } },
    tooltip: { trigger: 'axis' },
    grid: { left: '3%', right: '4%', bottom: '3%', containLabel: true },
    xAxis: { type: 'category', data: data.map(d => d.day), boundaryGap: false },
    yAxis: { type: 'value' },
    series: [{
      name: '请求数', type: 'line', smooth: true, areaStyle: { opacity: 0.3 },
      data: data.map(d => d.count), itemStyle: { color: '#18a058' }
    }]
  });
}

function renderChannelChart(data: Array<{ name: string; count: number }>) {
  if (!channelChartRef.value) return;
  if (!channelChart) channelChart = echarts.init(channelChartRef.value);
  channelChart.setOption({
    title: { text: '渠道使用分布', left: 'center', textStyle: { fontSize: 14 } },
    tooltip: { trigger: 'item', formatter: '{b}: {c}次 ({d}%)' },
    legend: { bottom: '5%' },
    series: [{
      type: 'pie', radius: ['40%', '70%'], avoidLabelOverlap: false,
      itemStyle: { borderRadius: 10, borderColor: '#fff', borderWidth: 2 },
      label: { show: false, position: 'center' },
      emphasis: { label: { show: true, fontSize: 18, fontWeight: 'bold' } },
      data: data.map(d => ({ name: d.name, value: d.count }))
    }]
  });
}

function renderTokenChart(data: Array<{ day: string; prompt_tokens: number; completion_tokens: number }>) {
  if (!tokenChartRef.value) return;
  if (!tokenChart) tokenChart = echarts.init(tokenChartRef.value);
  tokenChart.setOption({
    title: { text: 'Token 消耗（输入/输出）', left: 'center', textStyle: { fontSize: 14 } },
    tooltip: { trigger: 'axis', axisPointer: { type: 'shadow' } },
    legend: { bottom: 0 },
    grid: { left: '3%', right: '4%', bottom: '10%', containLabel: true },
    xAxis: { type: 'category', data: data.map(d => d.day) },
    yAxis: { type: 'value', name: 'Tokens' },
    series: [
      { name: '输入', type: 'bar', stack: 'token', data: data.map(d => d.prompt_tokens), itemStyle: { color: '#2080f0' } },
      { name: '输出', type: 'bar', stack: 'token', data: data.map(d => d.completion_tokens), itemStyle: { color: '#18a058' } }
    ]
  });
}

function renderUserRankChart(data: Array<{ api_key: string; count: number }>) {
  if (!userRankChartRef.value) return;
  if (!userRankChart) userRankChart = echarts.init(userRankChartRef.value);
  const names = data.map(d => d.api_key);
  const counts = data.map(d => d.count);
  userRankChart.setOption({
    title: { text: '用户调用排名 TOP 10', left: 'center', textStyle: { fontSize: 14 } },
    tooltip: { trigger: 'axis', axisPointer: { type: 'shadow' } },
    grid: { left: '3%', right: '4%', bottom: '3%', containLabel: true },
    xAxis: { type: 'value' },
    yAxis: { type: 'category', data: names, axisLabel: { fontSize: 11 } },
    series: [{
      name: '请求数', type: 'bar', data: counts,
      itemStyle: { color: '#f0a020' }
    }]
  });
}

function renderModelChart(data: Array<{ name: string; count: number }>) {
  if (!modelChartRef.value) return;
  if (!modelChart) modelChart = echarts.init(modelChartRef.value);
  modelChart.setOption({
    title: { text: '模型调用分布 TOP 10', left: 'center', textStyle: { fontSize: 14 } },
    tooltip: { trigger: 'item', formatter: '{b}: {c}次 ({d}%)' },
    legend: { bottom: '5%' },
    series: [{
      type: 'pie', radius: ['40%', '70%'], avoidLabelOverlap: false,
      itemStyle: { borderRadius: 10, borderColor: '#fff', borderWidth: 2 },
      label: { show: false, position: 'center' },
      emphasis: { label: { show: true, fontSize: 16, fontWeight: 'bold' } },
      data: data.map(d => ({ name: d.name, value: d.count }))
    }]
  });
}

function handleResize() {
  trendChart?.resize();
  channelChart?.resize();
  tokenChart?.resize();
  userRankChart?.resize();
  modelChart?.resize();
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
  userRankChart?.dispose();
  modelChart?.dispose();
});
</script>

<template>
  <NSpace vertical :size="16">
    <!-- Filter bar -->
    <NCard :bordered="false" class="card-wrapper">
      <NGrid :x-gap="12" :y-gap="8" responsive="screen" item-responsive>
        <NGi span="24 s:8 m:5">
          <NSelect v-model:value="filterChannel" :options="channelOptions" placeholder="按渠道筛选" clearable />
        </NGi>
        <NGi span="24 s:8 m:5">
          <NSelect v-model:value="filterModel" :options="modelOptions" placeholder="按模型筛选" clearable filterable />
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

    <!-- Stats Cards: 7 cards -->
    <NGrid :x-gap="16" :y-gap="16" responsive="screen" cols="1 s:2 m:3 xl:7" item-responsive>
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
          <NStatistic label="成功次数" :value="stats.success_count" />
        </NCard>
      </NGi>
      <NGi>
        <NCard :bordered="false" class="card-wrapper">
          <NStatistic label="失败次数" :value="stats.failure_count" />
        </NCard>
      </NGi>
      <NGi>
        <NCard :bordered="false" class="card-wrapper">
          <NStatistic label="P95 耗时" :value="stats.p95_latency_ms">
            <template #suffix>ms</template>
          </NStatistic>
        </NCard>
      </NGi>
      <NGi>
        <NCard :bordered="false" class="card-wrapper">
          <NStatistic label="总 Token" :value="stats.total_tokens" />
        </NCard>
      </NGi>
    </NGrid>

    <!-- Charts: Row 1 -->
    <NGrid :x-gap="16" :y-gap="16" responsive="screen" item-responsive>
      <NGi span="24 s:24 m:16">
        <NCard :bordered="false" class="card-wrapper">
          <template #header><span class="card-title">请求趋势</span></template>
          <div ref="trendChartRef" style="width: 100%; height: 350px"></div>
        </NCard>
      </NGi>
      <NGi span="24 s:24 m:8">
        <NCard :bordered="false" class="card-wrapper">
          <template #header><span class="card-title">渠道使用分布</span></template>
          <div ref="channelChartRef" style="width: 100%; height: 350px"></div>
        </NCard>
      </NGi>
    </NGrid>

    <!-- Charts: Row 2 -->
    <NGrid :x-gap="16" :y-gap="16" responsive="screen" item-responsive>
      <NGi span="24 s:24 m:12">
        <NCard :bordered="false" class="card-wrapper">
          <template #header><span class="card-title">用户调用排名 TOP 10</span></template>
          <div ref="userRankChartRef" style="width: 100%; height: 350px"></div>
        </NCard>
      </NGi>
      <NGi span="24 s:24 m:12">
        <NCard :bordered="false" class="card-wrapper">
          <template #header><span class="card-title">模型调用分布 TOP 10</span></template>
          <div ref="modelChartRef" style="width: 100%; height: 350px"></div>
        </NCard>
      </NGi>
    </NGrid>

    <!-- Charts: Row 3 -->
    <NCard :bordered="false" class="card-wrapper">
      <template #header><span class="card-title">Token 消耗（输入/输出）</span></template>
      <div ref="tokenChartRef" style="width: 100%; height: 350px"></div>
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
