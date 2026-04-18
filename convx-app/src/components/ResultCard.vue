<template>
  <div class="result-card" :class="{ 'result-card--success': result.status === 'completed' }">
    <template v-if="result.status === 'completed'">
      <div class="result-card__header">
        <q-icon
          name="sym_r_check_circle"
          size="32px"
          color="positive"
          class="result-card__icon"
        />
        <div>
          <div class="result-card__title">Conversion Complete</div>
          <div class="result-card__duration mono">
            {{ result.durationMs }}ms
          </div>
        </div>
      </div>

      <SizeComparisonBar
        v-if="result.outputSize != null"
        :input-size="result.inputSize"
        :output-size="result.outputSize"
      />

      <div class="result-card__actions">
        <q-btn
          v-if="isTauriEnv"
          flat
          no-caps
          size="sm"
          color="primary"
          icon="sym_r_folder_open"
          label="Show in Finder"
          @click="revealFile"
        />
        <q-btn
          flat
          no-caps
          size="sm"
          color="grey-5"
          icon="sym_r_refresh"
          label="Convert Another"
          @click="emit('convertAnother')"
        />
      </div>
    </template>

    <template v-else>
      <div class="result-card__header">
        <q-icon name="sym_r_error" size="32px" color="negative" />
        <div>
          <div class="result-card__title" style="color: var(--q-negative)">
            Conversion Failed
          </div>
          <div class="result-card__error">{{ result.error }}</div>
        </div>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { useQuasar } from 'quasar';
import { getBridge, isTauri } from 'src/services/bridge';
import type { ConversionResult } from 'src/types/conversion';
import { computed } from 'vue';
import SizeComparisonBar from './SizeComparisonBar.vue';

const props = defineProps<{
  result: ConversionResult;
}>();

const emit = defineEmits<{
  convertAnother: [];
}>();

const $q = useQuasar();
const isTauriEnv = computed(() => isTauri());

async function revealFile() {
  if (props.result.outputPath) {
    try {
      const bridge = await getBridge();
      await bridge.revealInFolder(props.result.outputPath);
    } catch (e) {
      console.error('Failed to reveal file:', e);
      $q.notify({
        type: 'negative',
        message: 'Could not open Finder for this file',
      });
    }
  }
}
</script>

<style lang="scss" scoped>
.result-card {
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding: 20px;
  border-radius: 14px;
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid rgba(255, 255, 255, 0.06);
  animation: fade-in 0.3s ease forwards;

  &--success {
    border-color: rgba($positive, 0.15);
  }

  &__header {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  &__icon {
    animation: bounce-check 0.4s ease forwards;
  }

  &__title {
    font-size: 15px;
    font-weight: 600;
    color: rgba(255, 255, 255, 0.87);
  }

  &__duration {
    font-size: 12px;
    color: rgba(255, 255, 255, 0.4);
    margin-top: 2px;
  }

  &__error {
    font-size: 13px;
    color: rgba(255, 255, 255, 0.5);
    margin-top: 4px;
  }

  &__actions {
    display: flex;
    gap: 8px;
    padding-top: 4px;
  }
}
</style>
