<template>
  <q-drawer
    v-model="drawerOpen"
    :mini="!expanded"
    :mini-to-overlay="true"
    :width="220"
    :mini-width="80"
    :style="isTauri ? 'margin-top: 40px; height: calc(100% - 40px)' : ''"
    :breakpoint="0"
    bordered
    class="sidebar"
    :class="{ 'sidebar--tauri': isTauri }"
    @mouseenter="expanded = true"
    @mouseleave="expanded = false"
  >
    <q-list class="sidebar__nav" padding>
      <q-item
        v-for="item in navItems"
        :key="item.route"
        :to="item.route"
        clickable
        :active="$route.name === item.name"
        active-class="sidebar__item--active"
        class="sidebar__item"
      >
        <q-item-section avatar>
          <q-icon :name="item.icon" size="24px" />
        </q-item-section>
        <q-item-section>{{ item.label }}</q-item-section>
      </q-item>
    </q-list>

    <q-space />

    <!-- Help link -->
    <q-list class="sidebar__nav" style="padding-bottom: 0">
      <q-item
        clickable
        class="sidebar__item"
        @click="openDocs"
      >
        <q-item-section avatar>
          <q-icon name="sym_r_help" size="24px" />
        </q-item-section>
        <q-item-section>Help</q-item-section>
      </q-item>
    </q-list>

    <!-- Enterprise indicator -->
    <div v-if="settings.enterpriseActive" class="sidebar__enterprise" :class="{ 'sidebar__enterprise--mini': !expanded }">
      <q-icon name="sym_r_verified" size="16px" color="orange" />
      <span v-if="expanded" class="sidebar__enterprise-label">Managed</span>
    </div>

    <!-- Mini mode: compact centered stats -->
    <div v-if="!expanded" class="sidebar__stats-mini">
      <div class="sidebar__stat-mini" title="Total conversions">
        <q-icon name="sym_r_swap_horiz" size="14px" />
        <span class="mono">{{ totalConversions }}</span>
      </div>
      <div
        class="sidebar__stat-mini"
        :title="isSpaceSavedPositive ? 'Space saved' : 'Net size increase'"
      >
        <q-icon name="sym_r_compress" size="14px" />
        <span class="mono" :class="spaceMetricClass">{{ spaceSavedShort }}</span>
      </div>
    </div>

    <!-- Expanded mode: full labels -->
    <div v-else class="sidebar__stats">
      <div class="sidebar__stat">
        <span class="sidebar__stat-value mono">{{ totalConversions }}</span>
        <span class="sidebar__stat-label">conversions</span>
      </div>
      <div class="sidebar__stat">
        <span class="sidebar__stat-value mono" :class="spaceMetricClass">{{ spaceSavedLabel }}</span>
        <span class="sidebar__stat-label">{{ spaceSavedSuffix }}</span>
      </div>
    </div>
  </q-drawer>
</template>

<script setup lang="ts">
import { useHistoryStore } from 'src/stores/history';
import { useSettingsStore } from 'src/stores/settings';
import { computed, ref } from 'vue';

const history = useHistoryStore();
const settings = useSettingsStore();
const drawerOpen = ref(true);
const expanded = ref(false);

const isTauri = !!(window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;

async function openDocs() {
  try {
    const { open } = await import('@tauri-apps/plugin-shell');
    await open('https://convx.dev/docs');
  } catch {
    window.open('https://convx.dev/docs', '_blank');
  }
}

const navItems = [
  { label: 'Convert', icon: 'sym_r_swap_horiz', route: '/', name: 'convert' },
  { label: 'History', icon: 'sym_r_history', route: '/history', name: 'history' },
  { label: 'Settings', icon: 'sym_r_settings', route: '/settings', name: 'settings' },
];

const totalConversions = computed(() => history.totalConversions);
const totalSpaceDelta = computed(() => history.totalSpaceSaved);
const isSpaceSavedPositive = computed(() => totalSpaceDelta.value >= 0);

function formatBytes(bytes: number, short = false): string {
  const abs = Math.abs(bytes);
  if (short) {
    if (abs < 1024) return `${abs}B`;
    if (abs < 1024 * 1024) return `${Math.round(abs / 1024)}K`;
    if (abs < 1024 * 1024 * 1024) return `${Math.round(abs / (1024 * 1024))}M`;
    return `${(abs / (1024 * 1024 * 1024)).toFixed(1)}G`;
  }

  if (abs < 1024) return `${abs} B`;
  if (abs < 1024 * 1024) return `${(abs / 1024).toFixed(1)} KB`;
  if (abs < 1024 * 1024 * 1024) return `${(abs / (1024 * 1024)).toFixed(1)} MB`;
  return `${(abs / (1024 * 1024 * 1024)).toFixed(1)} GB`;
}

const spaceSavedLabel = computed(() => {
  if (totalSpaceDelta.value <= 0) return '—';
  return formatBytes(totalSpaceDelta.value);
});

const spaceSavedShort = computed(() => {
  if (totalSpaceDelta.value <= 0) return '—';
  return formatBytes(totalSpaceDelta.value, true);
});

const spaceSavedSuffix = computed(() => 'saved');
const spaceMetricClass = computed(() =>
  totalSpaceDelta.value > 0 ? 'sidebar__metric--positive' : '',
);
</script>

<style lang="scss" scoped>
.sidebar {
  background: rgba(255, 255, 255, 0.02) !important;
  border-right: 1px solid rgba(255, 255, 255, 0.06) !important;

  &__nav {
    margin-top: 8px;
  }

  &__item {
    border-radius: 10px;
    margin: 2px 8px;
    color: rgba(255, 255, 255, 0.65);
    transition: all 0.2s ease;
    min-height: 44px;

    &:hover {
      color: rgba(255, 255, 255, 0.8);
      background: rgba(255, 255, 255, 0.04);
    }

    &--active {
      color: $primary !important;
      background: rgba($primary, 0.1) !important;
    }
  }

  // Compact stats for mini mode
  &__stats-mini {
    padding: 10px 0;
    border-top: 1px solid rgba(255, 255, 255, 0.06);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
  }

  &__stat-mini {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    color: rgba(255, 255, 255, 0.5);
    font-size: 11px;
  }

  // Full stats for expanded mode
  &__stats {
    padding: 12px 16px;
    border-top: 1px solid rgba(255, 255, 255, 0.06);
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  &__stat {
    display: flex;
    align-items: baseline;
    gap: 6px;
    white-space: nowrap;
  }

  &__stat-value {
    font-size: 13px;
    font-weight: 600;
    color: rgba(255, 255, 255, 0.7);
  }

  &__stat-label {
    font-size: 11px;
    color: rgba(255, 255, 255, 0.5);
  }

  &__metric--positive {
    color: rgba(84, 203, 114, 0.9) !important;
  }

  &__enterprise {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 16px;
    color: rgba(255, 180, 50, 0.8);
    font-size: 12px;
    font-weight: 500;

    &--mini {
      justify-content: center;
      padding: 8px 0;
    }
  }

  &__enterprise-label {
    white-space: nowrap;
  }
}
</style>
