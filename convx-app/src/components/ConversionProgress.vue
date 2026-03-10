<template>
  <div class="conv-progress" aria-live="polite" aria-atomic="true">
    <!-- Convert button (idle state) -->
    <button
      v-if="stage === 'idle'"
      class="conv-progress__btn"
      :disabled="!canConvert"
      @click="emit('convert')"
    >
      <q-icon name="sym_r_bolt" size="22px" />
      <span>Convert to {{ outputFormat.toUpperCase() }}</span>
    </button>

    <!-- Progress ring (active state) -->
    <div v-else-if="stage !== 'complete' && stage !== 'error'" class="conv-progress__ring">
      <svg viewBox="0 0 120 120" class="conv-progress__svg" aria-hidden="true">
        <defs>
          <linearGradient id="progressGrad" x1="0%" y1="0%" x2="100%" y2="0%">
            <stop offset="0%" stop-color="#3B82F6" />
            <stop offset="100%" stop-color="#8B5CF6" />
          </linearGradient>
        </defs>
        <circle
          cx="60" cy="60" r="52"
          fill="none"
          stroke="rgba(255,255,255,0.06)"
          stroke-width="6"
        />
        <circle
          cx="60" cy="60" r="52"
          fill="none"
          stroke="url(#progressGrad)"
          stroke-width="6"
          stroke-linecap="round"
          :stroke-dasharray="circumference"
          :stroke-dashoffset="dashOffset"
          transform="rotate(-90 60 60)"
          class="conv-progress__arc"
        />
      </svg>
      <div class="conv-progress__center">
        <span class="conv-progress__percent mono">{{ Math.round(percent) }}%</span>
        <span class="conv-progress__stage">{{ stageLabel }}</span>
      </div>
    </div>

    <!-- Success state -->
    <div v-else-if="stage === 'complete'" class="conv-progress__done">
      <q-icon name="sym_r_check_circle" size="48px" color="positive" class="conv-progress__check" />
      <span class="conv-progress__done-text">Done</span>
      <button class="conv-progress__again" :disabled="!canConvert" @click="emit('convert')">
        Convert Again
      </button>
    </div>

    <!-- Error state -->
    <div v-else-if="stage === 'error'" class="conv-progress__error">
      <q-icon name="sym_r_error" size="48px" color="negative" />
      <span class="conv-progress__error-text">Conversion failed</span>
      <span v-if="errorMessage" class="conv-progress__error-detail mono">{{ errorMessage }}</span>
      <button class="conv-progress__retry" :disabled="!canConvert" @click="emit('convert')">
        Retry
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import type { ConversionStage } from '../types/conversion';

const props = defineProps<{
  stage: ConversionStage;
  percent: number;
  outputFormat: string;
  canConvert: boolean;
  errorMessage?: string | null;
}>();

const emit = defineEmits<{
  convert: [];
}>();

const circumference = 2 * Math.PI * 52;
const dashOffset = computed(
  () => circumference - (props.percent / 100) * circumference,
);

const stageLabel = computed(() => {
  switch (props.stage) {
    case 'reading': return 'Reading...';
    case 'converting': return 'Converting...';
    case 'writing': return 'Writing...';
    default: return '';
  }
});
</script>

<style lang="scss" scoped>
.conv-progress {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  min-height: 160px;

  &__btn {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    width: 100%;
    padding: 14px 28px;
    border: none;
    border-radius: 14px;
    background: linear-gradient(135deg, $primary, $secondary);
    color: white;
    font-size: 15px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.25s ease;
    box-shadow: 0 4px 20px rgba($primary, 0.3);

    &:hover:not(:disabled) {
      transform: translateY(-2px);
      box-shadow: 0 8px 30px rgba($primary, 0.4);
    }

    &:active:not(:disabled) {
      transform: scale(0.98);
    }

    &:disabled {
      opacity: 0.4;
      cursor: not-allowed;
    }
  }

  &__ring {
    position: relative;
    width: 120px;
    height: 120px;
  }

  &__svg {
    width: 100%;
    height: 100%;
  }

  &__arc {
    transition: stroke-dashoffset 0.4s ease;
  }

  &__center {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
  }

  &__percent {
    font-size: 22px;
    font-weight: 700;
    color: white;
  }

  &__stage {
    font-size: 11px;
    color: rgba(255, 255, 255, 0.65);
    margin-top: 2px;
  }

  &__done {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
  }

  &__again {
    margin-top: 6px;
    padding: 7px 18px;
    border: 1px solid rgba($positive, 0.35);
    border-radius: 8px;
    background: rgba($positive, 0.12);
    color: $positive;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s ease;

    &:hover:not(:disabled) {
      background: rgba($positive, 0.2);
    }

    &:disabled {
      opacity: 0.45;
      cursor: not-allowed;
    }
  }

  &__check {
    animation: bounce-check 0.4s ease forwards;
  }

  &__done-text {
    font-size: 16px;
    font-weight: 600;
    color: $positive;
  }

  &__error {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
  }

  &__error-text {
    font-size: 14px;
    color: $negative;
  }

  &__error-detail {
    max-width: 280px;
    font-size: 11px;
    line-height: 1.35;
    color: rgba(255, 255, 255, 0.6);
    text-align: center;
    word-break: break-word;
  }

  &__retry {
    padding: 6px 20px;
    border: 1px solid rgba($negative, 0.3);
    border-radius: 8px;
    background: rgba($negative, 0.1);
    color: $negative;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s ease;

    &:hover {
      background: rgba($negative, 0.2);
    }
  }
}
</style>
