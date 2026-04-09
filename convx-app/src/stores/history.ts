import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import type { ConversionResult } from 'src/types/conversion';

const STORAGE_KEY = 'convx-history';
const MAX_HISTORY = 100;

export const useHistoryStore = defineStore('history', () => {
  const items = ref<ConversionResult[]>([]);

  const totalConversions = computed(() => items.value.length);
  const totalSpaceSaved = computed(() =>
    items.value.reduce((sum, r) => sum + (r.spaceSaved ?? 0), 0),
  );
  const successCount = computed(() =>
    items.value.filter((r) => r.status === 'completed').length,
  );

  function load() {
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (raw) items.value = JSON.parse(raw) as ConversionResult[];
    } catch {
      items.value = [];
    }
  }

  function persist() {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(items.value));
  }

  function add(result: ConversionResult) {
    items.value.unshift(result);
    if (items.value.length > MAX_HISTORY) {
      items.value = items.value.slice(0, MAX_HISTORY);
    }
    persist();
  }

  function clear() {
    items.value = [];
    persist();
  }

  load();

  return { items, totalConversions, totalSpaceSaved, successCount, add, clear };
});
