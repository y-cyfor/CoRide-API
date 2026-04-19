import { useAuthStore } from '@/store/modules/auth';
import { localStg } from '@/utils/storage';
import { fetchRefreshToken } from '../api';
import type { RequestInstanceState } from './type';

export function getAuthorization() {
  const token = localStg.get('token');
  const Authorization = token ? `Bearer ${token}` : null;

  return Authorization;
}

/** refresh token */
async function handleRefreshToken() {
  const refreshToken = localStg.get('refreshToken');
  if (!refreshToken) {
    const { resetStore } = useAuthStore();
    resetStore();
    return false;
  }

  try {
    const { data, error } = await fetchRefreshToken(refreshToken);
    if (error || !data) {
      const { resetStore } = useAuthStore();
      resetStore();
      return false;
    }

    // Update tokens
    localStg.set('token', data.token);
    if (data.refreshToken) {
      localStg.set('refreshToken', data.refreshToken);
    }
    return true;
  } catch {
    const { resetStore } = useAuthStore();
    resetStore();
    return false;
  }
}

export async function handleExpiredRequest(state: RequestInstanceState) {
  if (!state.refreshTokenPromise) {
    state.refreshTokenPromise = handleRefreshToken();
  }

  const success = await state.refreshTokenPromise;

  setTimeout(() => {
    state.refreshTokenPromise = null;
  }, 1000);

  return success;
}

export function showErrorMsg(state: RequestInstanceState, message: string) {
  if (!state.errMsgStack?.length) {
    state.errMsgStack = [];
  }

  const isExist = state.errMsgStack.includes(message);

  if (!isExist) {
    state.errMsgStack.push(message);

    window.$message?.error(message, {
      onLeave: () => {
        state.errMsgStack = state.errMsgStack.filter(msg => msg !== message);

        setTimeout(() => {
          state.errMsgStack = [];
        }, 5000);
      }
    });
  }
}
