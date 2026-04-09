<template>
  <q-page class="history-page" padding>
    <div class="history-page__container">
      <div class="history-page__header">
        <h1 class="history-page__title">History</h1>
        <q-btn
          v-if="history.items.length"
          flat
          no-caps
          size="sm"
          color="negative"
          label="Clear All"
          @click="history.clear()"
        />
      </div>

      <div v-if="!history.items.length" class="history-page__empty">
        <q-icon name="sym_r_history" size="64px" class="text-glass" />
        <div class="text-glass" style="font-size: 16px">No conversions yet</div>
        <div class="text-glass" style="font-size: 13px">
          Your conversion history will appear here
        </div>
      </div>

      <div v-else class="history-page__list">
        <div
          v-for="item in history.items"
          :key="item.id"
          class="history-item glass-card"
        >
          <div class="history-item__row">
            <q-icon
              :name="item.status === 'completed' ? 'sym_r_check_circle' : 'sym_r_error'"
              :color="item.status === 'completed' ? 'positive' : 'negative'"
              size="20px"
            />
            <div class="history-item__info">
              <span class="history-item__name mono">
                {{ fileName(item.inputPath) }}
              </span>
              <span class="text-glass" style="font-size: 12px">
                {{ item.inputFormat?.toUpperCase() }} → {{ item.outputFormat?.toUpperCase() }}
              </span>
            </div>
            <div class="history-item__stats">
              <span v-if="item.spaceSaved && item.spaceSaved > 0" class="history-item__saved">
                -{{ Math.round((item.spaceSaved / item.inputSize) * 100) }}%
              </span>
              <span class="mono" style="font-size: 12px; color: rgba(255,255,255,0.4)">
                {{ item.durationMs }}ms
              </span>
            </div>
          </div>
        </div>
      </div>
    </div>
  </q-page>
</template>

<script setup lang="ts">
import { useHistoryStore } from 'src/stores/history';

const history = useHistoryStore();

function fileName(path: string): string {
  return path.split('/').pop() || path;
}
</script>

<style lang="scss" scoped>
.history-page {
  position: relative;
  z-index: 1;
  padding: 24px 32px;

  &__container {
    max-width: 800px;
    margin: 0 auto;
  }

  &__header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 24px;
  }

  &__title {
    margin: 0;
    font-size: 22px;
    font-weight: 600;
    color: rgba(255, 255, 255, 0.85);
  }

  &__empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: 100px 0;
  }

  &__list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
}

.history-item {
  padding: 14px 16px;

  &__row {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  &__info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
    min-width: 0;
  }

  &__name {
    font-size: 13px;
    color: rgba(255, 255, 255, 0.8);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  &__stats {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  &__saved {
    font-size: 12px;
    font-weight: 600;
    color: $positive;
  }
}
</style>
