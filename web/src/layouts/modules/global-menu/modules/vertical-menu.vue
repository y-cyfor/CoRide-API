<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useRoute } from 'vue-router';
import { SimpleScrollbar } from '@sa/materials';
import { GLOBAL_SIDER_MENU_ID } from '@/constants/app';
import { useAppStore } from '@/store/modules/app';
import { useThemeStore } from '@/store/modules/theme';
import { useRouteStore } from '@/store/modules/route';
import { useRouterPush } from '@/hooks/common/router';
import { useMenu } from '../context';
import VersionFooter from '../components/version-footer.vue';

defineOptions({
  name: 'VerticalMenu'
});

const route = useRoute();
const appStore = useAppStore();
const themeStore = useThemeStore();
const routeStore = useRouteStore();
const { routerPushByKeyWithMetaQuery } = useRouterPush();
const { selectedKey } = useMenu();

const inverted = computed(() => !themeStore.darkMode && themeStore.sider.inverted);

const expandedKeys = ref<string[]>([]);

function updateExpandedKeys() {
  if (appStore.siderCollapse || !selectedKey.value) {
    expandedKeys.value = [];
    return;
  }
  expandedKeys.value = routeStore.getSelectedMenuKeyPath(selectedKey.value);
}

watch(
  () => route.name,
  () => {
    updateExpandedKeys();
  },
  { immediate: true }
);
</script>

<template>
  <Teleport :to="`#${GLOBAL_SIDER_MENU_ID}`">
    <div class="flex-col-stretch h-full">
      <SimpleScrollbar class="flex-1-hidden">
        <NMenu
          v-model:expanded-keys="expandedKeys"
          mode="vertical"
          :value="selectedKey"
          :collapsed="appStore.siderCollapse"
          :collapsed-width="themeStore.sider.collapsedWidth"
          :collapsed-icon-size="22"
          :options="routeStore.menus"
          :inverted="inverted"
          :indent="18"
          @update:value="routerPushByKeyWithMetaQuery"
        />
      </SimpleScrollbar>
      <VersionFooter v-if="!appStore.siderCollapse" />
    </div>
  </Teleport>
</template>

<style scoped></style>
