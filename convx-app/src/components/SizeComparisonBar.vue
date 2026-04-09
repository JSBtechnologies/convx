<template>
  <div class="size-bar">
    <div class="size-bar__track">
      <div
        class="size-bar__input"
        :style="{ width: '100%' }"
      />
      <div
        class="size-bar__output"
        :style="{ width: outputPercent + '%' }"
      />
    </div>
    <div class="size-bar__labels">
      <span class="size-bar__label">
        <span class="text-glass">Input:</span>
        <span class="mono">{{ formatSize(inputSize) }}</span>
      </span>
      <span class="size-bar__label">
        <span class="text-glass">Output:</span>
        <span class="mono">{{ formatSize(outputSize) }}</span>
      </span>
    </div>
    <div v-if="savings > 0" class="size-bar__savings">
      <q-icon name="sym_r_trending_down" size="14px" />
      {{ savingsPercent }}% smaller
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';

const props = defineProps<{
  inputSize: number;
  outputSize: number;
}>();

const savings = computed(() => props.inputSize - props.outputSize);
const savingsPercent = computed(() =>
  props.inputSize > 0
    ? Math.round((savings.value / props.inputSize) * 100)
    : 0,
);
const outputPercent = computed(() =>
  props.inputSize > 0
    ? Math.min(100, Math.round((props.outputSize / props.inputSize) * 100))
    : 0,
);

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}
</script>

<style lang="scss" scoped>
.size-bar {
  display: flex;
  flex-direction: column;
  gap: 8px;

  &__track {
    position: relative;
    height: 8px;
    border-radius: 4px;
    overflow: hidden;
    background: rgba(255, 255, 255, 0.06);
  }

  &__input {
    position: absolute;
    inset: 0;
    background: rgba(255, 255, 255, 0.1);
    border-radius: 4px;
  }

  &__output {
    position: absolute;
    top: 0;
    left: 0;
    bottom: 0;
    background: linear-gradient(90deg, $primary, $secondary);
    border-radius: 4px;
    animation: bar-fill 0.8s ease forwards;
  }

  &__labels {
    display: flex;
    justify-content: space-between;
    font-size: 12px;
    color: rgba(255, 255, 255, 0.7);
  }

  &__label {
    display: flex;
    gap: 4px;
  }

  &__savings {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 13px;
    font-weight: 600;
    color: $positive;
  }

}
</style>
