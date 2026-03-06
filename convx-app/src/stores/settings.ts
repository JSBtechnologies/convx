import { defineStore } from 'pinia';
import { ref, watch } from 'vue';
import type { EnterpriseSettings } from 'src/types/license';
import { getBridge } from 'src/services/bridge';

const STORAGE_KEY = 'convx-settings';

export const useSettingsStore = defineStore('settings', () => {
  const defaultQuality = ref(80);
  const defaultFormat = ref('webp');
  const outputDirectory = ref('');
  const overwriteExisting = ref(false);
  const showNotifications = ref(true);

  // Enterprise state
  const enterpriseActive = ref(false);
  const enterpriseLocked = ref(false);
  const allowedFormats = ref<string[]>([]);
  let suppressPersist = false;

  function load() {
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (raw) {
        const data = JSON.parse(raw) as Record<string, unknown>;
        defaultQuality.value = (data.defaultQuality as number) ?? 80;
        defaultFormat.value = (data.defaultFormat as string) ?? 'webp';
        outputDirectory.value = (data.outputDirectory as string) ?? '';
        overwriteExisting.value = (data.overwriteExisting as boolean) ?? false;
        showNotifications.value = (data.showNotifications as boolean) ?? true;
      }
    } catch {
      // use defaults
    }
  }

  function persist() {
    if (suppressPersist) return;
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({
        defaultQuality: defaultQuality.value,
        defaultFormat: defaultFormat.value,
        outputDirectory: outputDirectory.value,
        overwriteExisting: overwriteExisting.value,
        showNotifications: showNotifications.value,
      }),
    );
  }

  /** Apply enterprise settings overrides. Called after checking enterprise config.
   *  Suppresses persist to avoid saving enforced values to localStorage. */
  function applyEnterpriseOverrides(settings: EnterpriseSettings) {
    suppressPersist = true;
    enterpriseActive.value = true;
    enterpriseLocked.value = settings.locked;

    if (settings.default_quality !== undefined) defaultQuality.value = settings.default_quality;
    if (settings.default_format !== undefined) defaultFormat.value = settings.default_format;
    if (settings.output_directory !== undefined) outputDirectory.value = settings.output_directory;
    if (settings.overwrite_existing !== undefined) overwriteExisting.value = settings.overwrite_existing;
    if (settings.show_notifications !== undefined) showNotifications.value = settings.show_notifications;
    if (settings.allowed_formats !== undefined) allowedFormats.value = settings.allowed_formats;
    suppressPersist = false;
  }

  /** Check for enterprise config on startup. */
  async function checkEnterprise() {
    try {
      const bridge = await getBridge();
      const config = await bridge.getEnterpriseConfig();
      if (config.has_config && config.settings) {
        applyEnterpriseOverrides(config.settings);
      }
    } catch {
      // Not in enterprise mode or bridge not ready
    }
  }

  watch(
    [defaultQuality, defaultFormat, outputDirectory, overwriteExisting, showNotifications],
    persist,
    { deep: true },
  );

  load();

  return {
    defaultQuality, defaultFormat, outputDirectory,
    overwriteExisting, showNotifications,
    enterpriseActive, enterpriseLocked, allowedFormats,
    applyEnterpriseOverrides, checkEnterprise,
  };
});
