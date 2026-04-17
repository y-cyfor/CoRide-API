<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { NCard, NSpace, NDescriptions, NDescriptionsItem, NTag, NSpin, NSelect, NInputNumber, NButton, useMessage, NSwitch, NFormItem, NForm, NAlert } from 'naive-ui';
import { request } from '@/service/request';

const loading = ref(true);
const saving = ref(false);
const message = useMessage();

const config = ref<any>(null);
const logLevel = ref('info');
const retentionDays = ref(30);

// Global rate limit editable values
const rateLimitQps = ref(100);
const rateLimitConcurrency = ref(50);
const rateLimitAction = ref('reject');

const logLevelOptions = [
  { label: 'Trace', value: 'trace' },
  { label: 'Debug', value: 'debug' },
  { label: 'Info', value: 'info' },
  { label: 'Warn', value: 'warn' },
  { label: 'Error', value: 'error' }
];

const rateLimitActionOptions = [
  { label: '拒绝', value: 'reject' },
  { label: '排队', value: 'queue' }
];

async function loadConfig() {
  loading.value = true;
  try {
    const { data } = await request({
      url: '/admin/system/config',
      method: 'get'
    });
    if (data) {
      config.value = data;
      logLevel.value = data.log?.level || 'info';
      retentionDays.value = data.log?.retention_days || 30;
      rateLimitQps.value = data.global_rate_limit?.qps || 100;
      rateLimitConcurrency.value = data.global_rate_limit?.concurrency || 50;
      rateLimitAction.value = data.global_rate_limit?.action || 'reject';
    }
  } catch {
    message.error('加载配置失败');
  }
  loading.value = false;
}

async function updateLogLevel() {
  const { error } = await request({
    url: '/admin/system/config/log-level',
    method: 'put',
    data: { level: logLevel.value }
  });
  if (!error) {
    message.success('日志级别已更新为: ' + logLevel.value);
  } else {
    message.error('更新失败');
  }
}

async function updateRateLimit() {
  saving.value = true;
  const { error } = await request({
    url: '/admin/system/config/rate-limit',
    method: 'put',
    data: {
      qps: rateLimitQps.value,
      concurrency: rateLimitConcurrency.value,
      action: rateLimitAction.value
    }
  });
  if (!error) {
    message.success('全局限流已更新，重启后生效');
    await loadConfig();
  } else {
    message.error('更新失败');
  }
  saving.value = false;
}

onMounted(() => {
  loadConfig();
});
</script>

<template>
  <NSpace vertical :size="16">
    <NSpin :show="loading">
      <NSpace vertical :size="16" v-if="config">
        <!-- Server Config -->
        <NCard title="服务器配置" :bordered="false" class="card-wrapper">
          <NDescriptions :column="2" bordered label-placement="left">
            <NDescriptionsItem label="Host">
              <NTag type="info">{{ config.server.host }}</NTag>
            </NDescriptionsItem>
            <NDescriptionsItem label="Port">
              <NTag type="info">{{ config.server.port }}</NTag>
            </NDescriptionsItem>
          </NDescriptions>
        </NCard>

        <!-- Database Config -->
        <NCard title="数据库配置" :bordered="false" class="card-wrapper">
          <NDescriptions :column="2" bordered label-placement="left">
            <NDescriptionsItem label="数据库路径">
              <code>{{ config.database.path }}</code>
            </NDescriptionsItem>
            <NDescriptionsItem label="连接池大小">
              <NTag type="success">{{ config.database.pool_size }}</NTag>
            </NDescriptionsItem>
          </NDescriptions>
        </NCard>

        <!-- Log Config (editable) -->
        <NCard title="日志配置" :bordered="false" class="card-wrapper">
          <NSpace vertical :size="12">
            <NDescriptions :column="3" bordered label-placement="left">
              <NDescriptionsItem label="日志级别">
                <NSelect v-model:value="logLevel" :options="logLevelOptions" style="width: 120px" />
              </NDescriptionsItem>
              <NDescriptionsItem label="保留天数">
                <NInputNumber v-model:value="retentionDays" :min="1" :max="365" style="width: 120px" />
              </NDescriptionsItem>
              <NDescriptionsItem label="最大记录数">
                {{ config.log.max_records?.toLocaleString() || '100,000' }}
              </NDescriptionsItem>
            </NDescriptions>
            <NSpace justify="end">
              <NButton type="primary" size="small" @click="updateLogLevel">更新日志级别</NButton>
            </NSpace>
            <NAlert type="warning" size="small">修改后需重启服务才能生效</NAlert>
          </NSpace>
        </NCard>

        <!-- Proxy Config -->
        <NCard title="代理配置" :bordered="false" class="card-wrapper">
          <NDescriptions :column="3" bordered label-placement="left">
            <NDescriptionsItem label="超时时间">
              {{ config.proxy.timeout }}ms
            </NDescriptionsItem>
            <NDescriptionsItem label="最大重试">
              {{ config.proxy.max_retries }}
            </NDescriptionsItem>
            <NDescriptionsItem label="记录请求体">
              <NTag :type="config.proxy.log_request_body ? 'success' : 'default'">
                {{ config.proxy.log_request_body ? '是' : '否' }}
              </NTag>
            </NDescriptionsItem>
            <NDescriptionsItem label="记录响应体">
              <NTag :type="config.proxy.log_response_body ? 'success' : 'default'">
                {{ config.proxy.log_response_body ? '是' : '否' }}
              </NTag>
            </NDescriptionsItem>
          </NDescriptions>
        </NCard>

        <!-- Rate Limit Config (now editable) -->
        <NCard title="全局限流配置" :bordered="false" class="card-wrapper">
          <NSpace vertical :size="12">
            <NDescriptions :column="3" bordered label-placement="left">
              <NDescriptionsItem label="QPS 限制">
                <NInputNumber v-model:value="rateLimitQps" :min="1" style="width: 120px" />
              </NDescriptionsItem>
              <NDescriptionsItem label="并发限制">
                <NInputNumber v-model:value="rateLimitConcurrency" :min="1" style="width: 120px" />
              </NDescriptionsItem>
              <NDescriptionsItem label="超限动作">
                <NSelect v-model:value="rateLimitAction" :options="rateLimitActionOptions" style="width: 120px" />
              </NDescriptionsItem>
            </NDescriptions>
            <NSpace justify="end">
              <NButton type="primary" size="small" :loading="saving" @click="updateRateLimit">保存限流配置</NButton>
            </NSpace>
            <NAlert type="warning" size="small">修改后需重启服务才能生效</NAlert>
          </NSpace>
        </NCard>
      </NSpace>
      <div v-else>加载配置中...</div>
    </NSpin>
  </NSpace>
</template>

<style scoped>
code {
  font-family: 'Courier New', monospace;
  background: #f5f5f5;
  padding: 2px 6px;
  border-radius: 4px;
}
</style>
