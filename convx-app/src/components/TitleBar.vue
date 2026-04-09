<template>
  <div class="titlebar" :class="{ 'titlebar--mac': isMac }" data-tauri-drag-region>
    <!-- macOS: native traffic lights are provided by the OS via titleBarStyle overlay -->
    <div v-if="isMac" class="titlebar__traffic-spacer" />

    <div class="titlebar__center" data-tauri-drag-region>
      <span class="titlebar__logo" data-tauri-drag-region>CONVX</span>
    </div>

    <!-- Windows: buttons on the right -->
    <div v-if="!isMac" class="titlebar__controls">
      <button class="titlebar__btn" aria-label="Minimize" @click="minimize">
        <q-icon name="sym_r_minimize" size="18px" aria-hidden="true" />
      </button>
      <button class="titlebar__btn" aria-label="Maximize" @click="toggleMaximize">
        <q-icon name="sym_r_crop_square" size="16px" aria-hidden="true" />
      </button>
      <button class="titlebar__btn titlebar__btn--close" aria-label="Close" @click="close">
        <q-icon name="sym_r_close" size="18px" aria-hidden="true" />
      </button>
    </div>

    <!-- Spacer to balance layout on macOS -->
    <div v-if="isMac" class="titlebar__spacer" />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue';

const isMac = ref(false);

onMounted(() => {
  isMac.value = navigator.userAgent.includes('Mac');
});

async function minimize() {
  const { getCurrentWindow } = await import('@tauri-apps/api/window');
  getCurrentWindow().minimize();
}
async function toggleMaximize() {
  const { getCurrentWindow } = await import('@tauri-apps/api/window');
  getCurrentWindow().toggleMaximize();
}
async function close() {
  const { getCurrentWindow } = await import('@tauri-apps/api/window');
  getCurrentWindow().close();
}
</script>

<style lang="scss" scoped>
.titlebar {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  height: 40px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 16px;
  background: rgba(255, 255, 255, 0.03);
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  z-index: 9999;
  user-select: none;
  -webkit-user-select: none;

  &__center {
    flex: 1;
    display: flex;
    justify-content: center;
  }

  &__logo {
    font-size: 13px;
    font-weight: 700;
    letter-spacing: 0.5px;
    color: rgba(255, 255, 255, 0.65);
  }

  // Spacer to account for native macOS traffic lights
  &__traffic-spacer {
    width: 78px;
    flex-shrink: 0;
  }

  &__spacer {
    width: 78px;
    flex-shrink: 0;
  }

  // Windows controls
  &__controls {
    display: flex;
    gap: 2px;
  }

  &__btn {
    width: 36px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
    background: transparent;
    color: rgba(255, 255, 255, 0.6);
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.15s ease;

    &:hover {
      background: rgba(255, 255, 255, 0.08);
      color: rgba(255, 255, 255, 0.9);
    }

    &--close:hover {
      background: rgba(239, 68, 68, 0.8);
      color: white;
    }
  }
}

</style>
