<template>
  <q-layout view="hHh lpR fFf" class="main-layout">
    <TitleBar v-if="isTauriEnv && isMac" />

    <nav aria-label="Main navigation">
      <AppSidebar />
    </nav>

    <q-page-container :style="{ 'padding-top': isTauriEnv && isMac ? '40px' : '0' }" role="main">
      <!-- Animated background blobs -->
      <div class="background-blobs" aria-hidden="true">
        <div class="blob blob--1" />
        <div class="blob blob--2" />
        <div class="blob blob--3" />
      </div>

      <router-view />
    </q-page-container>
  </q-layout>
</template>

<script setup lang="ts">
import AppSidebar from 'src/components/AppSidebar.vue';
import TitleBar from 'src/components/TitleBar.vue';
import { computed } from 'vue';

const isTauriEnv = computed(
  () => !!((window as unknown as Record<string, unknown>).__TAURI_INTERNALS__),
);
const isMac = computed(() => navigator.userAgent.includes('Mac'));
</script>

<style lang="scss" scoped>
.main-layout {
  background: $dark;
  min-height: 100vh;
}

.background-blobs {
  position: fixed;
  inset: 0;
  pointer-events: none;
  overflow: hidden;
  z-index: 0;
}

.blob {
  position: absolute;
  border-radius: 50%;
  filter: blur(80px);
  opacity: 0.07;

  &--1 {
    width: 500px;
    height: 500px;
    background: $primary;
    top: -120px;
    right: -100px;
    animation: blob-float 25s ease-in-out infinite alternate;
  }

  &--2 {
    width: 350px;
    height: 350px;
    background: $secondary;
    bottom: -80px;
    left: -60px;
    animation: blob-float 20s ease-in-out infinite alternate-reverse;
  }

  &--3 {
    width: 280px;
    height: 280px;
    background: $accent;
    top: 40%;
    left: 40%;
    animation: blob-float 30s ease-in-out infinite alternate;
  }
}
</style>
