<template>
  <q-dialog
    :model-value="modelValue"
    persistent
    maximized
    transition-show="fade"
    transition-hide="fade"
  >
    <q-card class="activation-card" aria-labelledby="activation-dialog-title">
      <!-- Input state -->
      <div v-if="state === 'input'" class="activation-body">
        <div class="activation-header">
          <div id="activation-dialog-title" class="activation-title">Activate convx</div>
          <div class="activation-subtitle text-glass">
            Enter your license key to get started.
          </div>
        </div>

        <q-input
          v-model="licenseKey"
          outlined
          dark
          label="License key"
          placeholder="CONVX-XXXX-XXXX-XXXX-XXXX"
          class="activation-input"
          :error="!!inputError"
          :error-message="inputError"
          @update:model-value="onKeyInput"
          @keydown.enter="isKeyValid && doActivate()"
        >
          <template v-slot:prepend>
            <q-icon name="sym_r_key" />
          </template>
        </q-input>

        <q-btn
          color="primary"
          no-caps
          label="Activate"
          class="full-width"
          :disable="!isKeyValid"
          :loading="activating"
          @click="doActivate"
        />

        <div class="activation-purchase text-glass">
          Don't have a key?
          <a href="#" class="activation-link" @click.prevent="openPurchase">
            Purchase a license
          </a>
        </div>
      </div>

      <!-- Conflict state (key active on another device) -->
      <div v-else-if="state === 'conflict'" class="activation-body activation-centered">
        <q-icon name="sym_r_warning" color="warning" size="48px" />
        <div class="activation-title">Already activated</div>
        <div class="text-glass" style="max-width: 400px; text-align: center">
          This key is currently active on <strong>{{ conflictDevice }}</strong>.
          Transfer it to this device? The other device will be deactivated.
        </div>
        <div class="activation-actions">
          <q-btn
            color="primary"
            no-caps
            label="Transfer to this device"
            :loading="transferring"
            @click="doTransfer"
          />
          <q-btn
            flat
            color="primary"
            no-caps
            label="Use a different key"
            @click="state = 'input'"
          />
        </div>
      </div>

      <!-- Success state -->
      <div v-else-if="state === 'success'" class="activation-body activation-centered">
        <q-icon name="sym_r_check_circle" color="positive" size="48px" />
        <div class="activation-title">You're all set</div>
        <div class="text-glass">convx is activated on {{ activatedDevice }}.</div>
        <q-btn color="positive" no-caps label="Start Converting" @click="close" />
      </div>

      <!-- Error state -->
      <div v-else-if="state === 'error'" class="activation-body activation-centered">
        <q-icon name="sym_r_error" color="negative" size="48px" />
        <div class="activation-title">Activation failed</div>
        <div class="text-glass" style="max-width: 400px; text-align: center">
          {{ errorMessage }}
        </div>
        <q-btn color="primary" no-caps label="Try again" @click="state = 'input'" />
      </div>
    </q-card>
  </q-dialog>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue';
import { getBridge } from '../services/bridge';

const props = defineProps<{
  modelValue: boolean;
}>();

const emit = defineEmits<{
  'update:modelValue': [value: boolean];
  activated: [];
}>();

type ActivationState = 'input' | 'conflict' | 'success' | 'error' | 'auto_activating';

const state = ref<ActivationState>('auto_activating');
const licenseKey = ref('');
const inputError = ref('');
const activating = ref(false);
const transferring = ref(false);
const conflictDevice = ref('');
const activatedDevice = ref('');
const errorMessage = ref('');

// Try silent/auto activation on mount
import { onMounted } from 'vue';
onMounted(async () => {
  try {
    const bridge = await getBridge();
    const result = await bridge.autoActivate();

    if (result.outcome === 'activated') {
      activatedDevice.value = result.device_name ?? 'this device';
      state.value = 'success';
      return;
    }

    if (result.outcome === 'already_active') {
      // Already activated on this device — just proceed
      activatedDevice.value = result.device_name ?? 'this device';
      state.value = 'success';
      return;
    }
  } catch {
    // Auto-activation not available, fall through to manual
  }

  state.value = 'input';
});

const KEY_PATTERN = /^CONVX-[A-Z0-9]{4}-[A-Z0-9]{4}-[A-Z0-9]{4}-[A-Z0-9]{4}$/;

const isKeyValid = computed(() => KEY_PATTERN.test(licenseKey.value));

async function openPurchase() {
  try {
    const { open } = await import('@tauri-apps/plugin-shell');
    await open('https://convx.dev');
  } catch {
    window.open('https://convx.dev', '_blank');
  }
}

function onKeyInput(raw: string | number | null) {
  if (raw == null) return;
  inputError.value = '';
  const str = String(raw);

  // Auto-format: strip non-alphanumeric, uppercase, insert dashes
  const clean = str.replace(/[^A-Za-z0-9]/g, '').toUpperCase().slice(0, 21);

  if (clean.length <= 5) {
    licenseKey.value = clean;
  } else {
    const prefix = clean.slice(0, 5);
    const rest = clean.slice(5);
    const groups = rest.match(/.{1,4}/g) || [];
    licenseKey.value = [prefix, ...groups].join('-');
  }
}

async function doActivate() {
  if (!isKeyValid.value) return;
  activating.value = true;
  inputError.value = '';

  try {
    const bridge = await getBridge();
    const result = await bridge.activateLicense(licenseKey.value);

    switch (result.outcome) {
      case 'activated':
        activatedDevice.value = result.device_name ?? 'this device';
        state.value = 'success';
        break;
      case 'already_active':
        conflictDevice.value = result.device_name ?? 'another device';
        state.value = 'conflict';
        break;
      case 'error':
        errorMessage.value = (result as { outcome: 'error'; message: string }).message ?? 'Unknown error';
        state.value = 'error';
        break;
    }
  } catch (err) {
    errorMessage.value = err instanceof Error ? err.message : String(err);
    state.value = 'error';
  } finally {
    activating.value = false;
  }
}

async function doTransfer() {
  transferring.value = true;

  try {
    const bridge = await getBridge();
    await bridge.transferLicense(licenseKey.value);
    activatedDevice.value = 'this device';
    state.value = 'success';
  } catch (err) {
    errorMessage.value = err instanceof Error ? err.message : String(err);
    state.value = 'error';
  } finally {
    transferring.value = false;
  }
}

function close() {
  emit('activated');
  emit('update:modelValue', false);
}
</script>

<style scoped lang="scss">
.activation-card {
  min-height: 100vh;
  border-radius: 0;
  padding: 32px;
  position: relative;
  background: radial-gradient(circle at 70% 10%, rgba($primary, 0.18), transparent 45%), $dark;
}

.activation-body {
  max-width: 480px;
  display: flex;
  flex-direction: column;
  gap: 16px;
  align-items: center;
  text-align: center;
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
}

.activation-header {
  margin-bottom: 8px;
}

.activation-title {
  font-size: 28px;
  font-weight: 700;
  color: white;
}

.activation-subtitle {
  margin-top: 6px;
}

.activation-input {
  margin-top: 8px;
  width: 100%;

  :deep(.q-field__control) {
    font-family: monospace;
    font-size: 15px;
    letter-spacing: 1px;
  }
}

.activation-actions {
  display: flex;
  flex-direction: column;
  gap: 8px;
  align-items: center;
}

.activation-purchase {
  margin-top: 8px;
  font-size: 13px;
}

.activation-link {
  color: var(--q-primary);
  text-decoration: none;

  &:hover {
    text-decoration: underline;
  }
}
</style>
