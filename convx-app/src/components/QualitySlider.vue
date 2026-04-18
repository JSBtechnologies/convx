<template>
  <div class="quality-slider">
    <div class="quality-slider__header">
      <span class="quality-slider__label">Quality</span>
      <span class="quality-slider__value mono">{{ modelValue }}%</span>
    </div>

    <q-slider
      :model-value="modelValue"
      @update:model-value="emit('update:modelValue', $event as number)"
      :min="10"
      :max="100"
      :step="5"
      color="primary"
      track-size="4px"
      thumb-size="16px"
      class="quality-slider__track"
      aria-label="Quality"
      :aria-valuetext="modelValue + '% quality'"
    />

    <div class="quality-slider__presets">
      <button
        v-for="preset in presets"
        :key="preset.value"
        type="button"
        :aria-pressed="modelValue === preset.value"
        :aria-label="preset.label + ' quality (' + preset.value + '%)'"
        class="quality-slider__preset"
        :class="{ 'quality-slider__preset--active': modelValue === preset.value }"
        @click="emit('update:modelValue', preset.value)"
      >
        {{ preset.label }}
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  modelValue: number;
}>();

const emit = defineEmits<{
  'update:modelValue': [value: number];
}>();

const presets = [
  { label: 'Web', value: 75 },
  { label: 'High', value: 90 },
  { label: 'Max', value: 100 },
];
</script>

<style lang="scss" scoped>
.quality-slider {
  display: flex;
  flex-direction: column;
  gap: 8px;

  &__header {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
  }

  &__label {
    font-size: 13px;
    font-weight: 500;
    color: rgba(255, 255, 255, 0.7);
  }

  &__value {
    font-size: 14px;
    font-weight: 700;
    color: $primary;
  }

  &__track {
    padding: 0 4px;
  }

  &__presets {
    display: flex;
    gap: 6px;
  }

  &__preset {
    flex: 1;
    padding: 6px 8px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.03);
    color: rgba(255, 255, 255, 0.65);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s ease;
    text-align: center;

    &:hover {
      border-color: rgba($primary, 0.3);
      color: rgba(255, 255, 255, 0.7);
    }

    &--active {
      border-color: $primary;
      background: rgba($primary, 0.1);
      color: white;
    }
  }
}
</style>
