import { ref } from 'vue';

interface VersionInfo {
  currentVersion: string;
  latestVersion: string | null;
  hasUpdate: boolean;
  releaseUrl: string;
  loading: boolean;
}

const REPO_URL = 'https://github.com/y-cyfor/CoRide-API';
const API_URL = 'https://api.github.com/repos/y-cyfor/CoRide-API/releases/latest';

/**
 * Compare two semver strings. Returns 1 if a > b, -1 if a < b, 0 if equal.
 */
function compareSemver(a: string, b: string): number {
  const parse = (v: string) => v.replace(/^v/, '').split('.').map(Number);
  const [a1, a2, a3] = parse(a);
  const [b1, b2, b3] = parse(b);
  if (a1 !== b1) return a1 > b1 ? 1 : -1;
  if (a2 !== b2) return a2 > b2 ? 1 : -1;
  if (a3 !== b3) return a3 > b3 ? 1 : -1;
  return 0;
}

export function useVersionCheck() {
  const versionInfo = ref<VersionInfo>({
    currentVersion: PACKAGE_VERSION || '0.0.0',
    latestVersion: null,
    hasUpdate: false,
    releaseUrl: REPO_URL + '/releases',
    loading: false
  });

  async function checkUpdate() {
    versionInfo.value.loading = true;
    try {
      const res = await fetch(API_URL, {
        headers: { Accept: 'application/vnd.github.v3+json' }
      });
      if (!res.ok) return;

      const data = await res.json();
      const latest = data.tag_name || data.name;
      if (latest) {
        versionInfo.value.latestVersion = latest.replace(/^v/, '');
        versionInfo.value.hasUpdate = compareSemver(latest, versionInfo.value.currentVersion) > 0;
        versionInfo.value.releaseUrl = data.html_url || REPO_URL + '/releases';
      }
    } catch {
      // Silently fail - network issues shouldn't break the app
    } finally {
      versionInfo.value.loading = false;
    }
  }

  /**
   * Schedule version checks:
   * - Run immediately
   * - Then at every hour :00:00
   */
  function startAutoCheck() {
    checkUpdate();

    const now = new Date();
    const msToNextHour = (60 - now.getMinutes()) * 60 * 1000 - now.getSeconds() * 1000 - now.getMilliseconds() + 1000;

    setTimeout(() => {
      checkUpdate();
      setInterval(checkUpdate, 3600000);
    }, msToNextHour);
  }

  return { versionInfo, checkUpdate, startAutoCheck };
}
