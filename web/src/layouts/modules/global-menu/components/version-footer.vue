<script setup lang="ts">
import { onMounted } from 'vue';
import { useVersionCheck } from '@/hooks/common/version';

defineOptions({
  name: 'VersionFooter'
});

const { versionInfo, startAutoCheck } = useVersionCheck();

onMounted(() => {
  startAutoCheck();
});
</script>

<template>
  <div class="version-footer">
    <a
      v-if="versionInfo.hasUpdate"
      :href="versionInfo.releaseUrl"
      target="_blank"
      rel="noopener noreferrer"
      class="version-link has-update"
    >
      <span class="current">v{{ versionInfo.currentVersion }}</span>
      <span class="arrow">→</span>
      <span class="latest">v{{ versionInfo.latestVersion }}</span>
    </a>
    <span v-else class="version-link">
      <span>v{{ versionInfo.currentVersion }}</span>
    </span>
  </div>
</template>

<style scoped>
.version-footer {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 8px 0;
  font-size: 12px;
  color: rgba(255, 255, 255, 0.45);
  border-top: 1px solid rgba(255, 255, 255, 0.08);
  flex-shrink: 0;
}

.version-link {
  display: flex;
  align-items: center;
  gap: 4px;
  text-decoration: none;
  transition: color 0.2s;
}

.version-link:hover {
  color: rgba(255, 255, 255, 0.85);
}

.version-link.has-update {
  color: rgba(255, 255, 255, 0.65);
}

.version-link.has-update:hover {
  color: #18a058;
}

.latest {
  color: #18a058;
  font-weight: 600;
}

.arrow {
  opacity: 0.5;
  font-size: 10px;
}
</style>
