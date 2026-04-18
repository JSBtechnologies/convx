import { defineStore } from 'pinia';
import { ref, watch } from 'vue';

const STORAGE_KEY = 'convx-settings';

export const useSettingsStore = defineStore('settings', () => {
  const defaultQuality = ref(80);
  const defaultFormat = ref('webp');
  const outputDirectory = ref('');
  const overwriteExisting = ref(false);
  const showNotifications = ref(true);

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

  watch(
    [defaultQuality, defaultFormat, outputDirectory, overwriteExisting, showNotifications],
    persist,
    { deep: true },
  );

  load();

  return {
    defaultQuality, defaultFormat, outputDirectory,
    overwriteExisting, showNotifications,
  };
});
