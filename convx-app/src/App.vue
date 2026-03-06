<template>
  <div>
    <router-view />
    <LicenseActivation
      v-model="showActivation"
      @activated="onLicenseActivated"
    />
    <DependencySetupWizard
      v-model="showDependencyWizard"
      @ready="onDependenciesReady"
    />
  </div>
</template>

<script setup lang="ts">
import { Notify } from 'quasar';
import { defineAsyncComponent, onMounted, ref } from 'vue';
import { createBridge, getBridge, isTauri } from './services/bridge';

const LicenseActivation = defineAsyncComponent(
  () => import('./components/LicenseActivation.vue').then((m) => (m as { default: unknown }).default as never),
);

const DependencySetupWizard = defineAsyncComponent(
  () => import('./components/DependencySetupWizard.vue').then((m) => (m as { default: unknown }).default as never),
);

const showActivation = ref(false);
const showDependencyWizard = ref(false);

onMounted(async () => {
  await createBridge();

  if (!isTauri()) return;

  try {
    const bridge = await getBridge();

    // License check first
    const licenseStatus = await bridge.checkLicense();
    if (licenseStatus.status === 'not_activated') {
      showActivation.value = true;
      return;
    }

    // Ensure post-install setup completed (CLI symlinks, venv, pip modules)
    // This silently repairs anything the .pkg postinstall missed
    try {
      const postInstall = await bridge.ensurePostInstall();
      if (!postInstall.ok) {
        console.warn('[convx] Post-install repairs needed:', postInstall.repairs);
      }
    } catch (e) {
      console.warn('[convx] Post-install check failed:', e);
    }

    // Then dependency check
    const deps = await bridge.checkDependencies();
    showDependencyWizard.value = !deps.ok;
  } catch {
    showDependencyWizard.value = true;
  }
});

function onLicenseActivated() {
  showActivation.value = false;
  // Ensure post-install setup, then check dependencies
  getBridge()
    .then(async (bridge) => {
      try { await bridge.ensurePostInstall(); } catch { /* non-fatal */ }
      return bridge.checkDependencies();
    })
    .then((status) => {
      showDependencyWizard.value = !status.ok;
    })
    .catch(() => {
      showDependencyWizard.value = true;
    });
}

function onDependenciesReady() {
  showDependencyWizard.value = false;
  Notify.create({ type: 'positive', message: 'Dependencies verified. Ready to convert.' });
}
</script>
