<script setup lang="ts">
import { onMounted } from 'vue';
import { useVersionCheck } from '@/hooks/common/version';

defineOptions({
  name: 'VersionFooter'
});

defineProps<{
  inverted?: boolean;
}>();

const { versionInfo, startAutoCheck } = useVersionCheck();

onMounted(() => {
  startAutoCheck();
});
</script>

<template>
  <a
    class="version-footer"
    :class="{ inverted, 'has-update': versionInfo.hasUpdate }"
    :href="versionInfo.releaseUrl"
    target="_blank"
    rel="noopener noreferrer"
  >
    <template v-if="versionInfo.hasUpdate">
      <span class="current">v{{ versionInfo.currentVersion }}（有更新）</span>
    </template>
    <template v-else>
      <span>v{{ versionInfo.currentVersion }}</span>
    </template>
  </a>
</template>

<style scoped>
.version-footer {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 8px 0;
  font-size: 12px;
  text-decoration: none;
  border-top: 1px solid rgba(128, 128, 128, 0.15);
  flex-shrink: 0;
  transition: color 0.2s;
}

/* Dark/inverted sidebar: light text */
.version-footer.inverted {
  color: rgba(255, 255, 255, 0.45);
}

.version-footer.inverted:hover {
  color: rgba(255, 255, 255, 0.85);
}

/* Light sidebar: dark text */
.version-footer:not(.inverted) {
  color: rgba(0, 0, 0, 0.45);
}

.version-footer:not(.inverted):hover {
  color: rgba(0, 0, 0, 0.85);
}

/* Has update: red text */
.version-footer.has-update {
  color: #e80c47 !important;
  font-weight: 600;
}

.version-footer.has-update:hover {
  color: #c0093a !important;
}
</style>
