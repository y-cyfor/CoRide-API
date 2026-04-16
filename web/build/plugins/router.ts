import type { RouteMeta } from 'vue-router';
import ElegantVueRouter from '@elegant-router/vue/vite';
import type { RouteKey } from '@elegant-router/types';

const routeTitleMap: Record<string, string> = {
  home: '仪表盘',
  manage: '管理中心',
  routing: '请求分流',
  'routing_app-profile': '应用预设',
  'routing_traffic-plan': '应用方案',
  upstream: '上游模型',
  upstream_channel: '渠道管理',
  upstream_model: '模型管理',
  control: '流量控制',
  control_ratelimit: '限流管理',
  control_quota: '配额管理',
  control_user: '用户管理',
  data: '数据统计',
  data_log: '请求日志',
  data_stats: '使用统计',
  settings: '系统设置',
  'system-settings': '系统设置',
  system_settings: '系统设置'
};

const routeOrderMap: Record<string, number> = {
  home: 1,
  routing: 2,
  upstream: 3,
  control: 4,
  data: 5,
  settings: 6
};

export function setupElegantRouter() {
  return ElegantVueRouter({
    layouts: {
      base: 'src/layouts/base-layout/index.vue',
      blank: 'src/layouts/blank-layout/index.vue'
    },
    routePathTransformer(routeName, routePath) {
      const key = routeName as RouteKey;

      if (key === 'login') {
        const modules: UnionKey.LoginModule[] = ['pwd-login', 'code-login', 'register', 'reset-pwd', 'bind-wechat'];

        const moduleReg = modules.join('|');

        return `/login/:module(${moduleReg})?`;
      }

      return routePath;
    },
    onRouteMetaGen(routeName) {
      const key = routeName as RouteKey;

      const constantRoutes: RouteKey[] = ['login', '403', '404', '500'];

      const meta: Partial<RouteMeta> = {
        title: routeTitleMap[key] || key,
        i18nKey: `route.${key}` as App.I18n.I18nKey
      };

      if (routeOrderMap[key] !== undefined) {
        meta.order = routeOrderMap[key];
      }

      if (constantRoutes.includes(key)) {
        meta.constant = true;
        meta.hideInMenu = true;
      }

      if (key === 'iframe-page') {
        meta.hideInMenu = true;
      }

      return meta;
    }
  });
}
