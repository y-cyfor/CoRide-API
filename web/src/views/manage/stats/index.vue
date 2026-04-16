<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { NCard, NSpace, NStatistic, NTag, NSelect, NButton, NGrid, NGi, NInputNumber } from 'naive-ui';
import { fetchUsageStats, fetchDashboardStats, fetchChannelList, fetchModelList } from '@/service/api';
import * as echarts from 'echarts';
import type { EChartsOption } from 'echarts';

const loading = ref(false);
const stats = ref({
  total_requests: 0,
  today_requests: 0,
  active_users: 0,
  total_tokens: 0
});
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

  const [dashRes, usageRes] = await Promise.all([
    fetchDashboardStats(),
    fetchUsageStats({
      channel_id: filterChannel.value,
      model: filterModel.value || undefined,
      days: filterDays.value
    })
  ]);

  if (dashRes.data) {
    stats.value = {
      total_requests: dashRes.data.total_requests,
      today_requests: dashRes.data.today_requests,
      active_users: dashRes.data.active_users,
      total_tokens: usageRes.data?.total_tokens || 0
    };
  }

  if (usageRes.data) {
    renderTrendChart(usageRes.data.daily_trend);
    renderChannelChart(usageRes.data.channel_usage);
    renderTokenChart(usageRes.data.token_daily || []);
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
  if (!trendChart) {
    trendChart = echarts.init(trendChartRef.value);
  }

  const option: EChartsOption = {
    title: { text: '近7天请求趋势', left: 'center' },
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
  if (!channelChart) {
    channelChart = echarts.init(channelChartRef.value);
  }

  const option: EChartsOption = {
    title: { text: '渠道使用分布', left: 'center' },
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
  if (!tokenChart) {
    tokenChart = echarts.init(tokenChartRef.value);
  }

  const option: EChartsOption = {
    title: { text: '近7天 Token 消耗（输入/输出）', left: 'center' },
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
    <NGrid :x-gap="16" :y-gap="16" responsive="screen" cols="1 s:2 m:4" item-responsive>
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
          <NStatistic label="总Token消耗" :value="stats.total_tokens" />
        </NCard>
      </NGi>
    </NGrid>

    <!-- Charts -->
    <NGrid :x-gap="16" :y-gap="16" responsive="screen" item-responsive>
      <NGi span="24 s:24 m:16">
        <NCard :bordered="false" class="card-wrapper">
          <template #header><span class="card-title">近7天请求趋势</span></template>
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

    <!-- Token consumption chart -->
    <NCard :bordered="false" class="card-wrapper">
      <template #header><span class="card-title">近7天 Token 消耗（输入/输出）</span></template>
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
