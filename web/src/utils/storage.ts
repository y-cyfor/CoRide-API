import { createLocalforage, createStorage } from '@sa/utils';

const storagePrefix = import.meta.env.VITE_STORAGE_PREFIX || '';

export const localStg = createStorage<StorageType.Local>('local', storagePrefix);

export const sessionStg = createStorage<StorageType.Session>('session', storagePrefix);

export const localforage = createLocalforage<StorageType.Local>('local');

/** Storage key constants to avoid hardcoded strings across the codebase. */
export const STG_KEY = {
  TOKEN: 'token',
  REFRESH_TOKEN: 'refreshToken',
} as const;
