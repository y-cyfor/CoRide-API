import { useAuthStore } from '@/store/modules/auth';

export function useAuth() {
  const authStore = useAuthStore();

  function hasRole(role: string) {
    if (!authStore.isLogin) {
      return false;
    }

    return authStore.userInfo.role === role;
  }

  return {
    hasRole
  };
}
