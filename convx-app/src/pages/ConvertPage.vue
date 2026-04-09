<template>
  <q-page class="convert-page" padding>
    <!-- Empty state: drop zone -->
    <FileDropZone
      v-if="!store.hasFile"
      @file-selected="onFileSelected"
    />

    <!-- File loaded: three-panel layout -->
    <div v-else class="convert-page__panels">
      <!-- LEFT: Input Preview -->
      <div class="convert-page__panel glass-card">
        <div class="panel-header">
          <q-icon :name="categoryIcon" size="20px" color="primary" />
          <span>Input</span>
        </div>
        <div class="file-info">
          <q-icon name="sym_r_description" size="48px" class="file-info__icon" />
          <div class="file-info__name mono">{{ store.inputFile?.name }}</div>
          <div class="file-info__meta text-glass">
            <span class="mono">{{ formatSize(store.inputFile?.size ?? 0) }}</span>
            <span class="file-info__badge">{{ store.inputFile?.extension?.toUpperCase() }}</span>
          </div>
        </div>
        <q-btn
          flat
          no-caps
          dense
          size="sm"
          color="grey-5"
          icon="sym_r_close"
          label="Remove"
          class="q-mt-auto"
          @click="store.clearInputFile()"
        />
      </div>

      <!-- CENTER: Options -->
      <div class="convert-page__panel convert-page__panel--center glass-card">
        <div class="panel-header">
          <q-icon name="sym_r_tune" size="20px" color="primary" />
          <span>Options</span>
        </div>

        <FormatSelector
          v-model="store.outputFormat"
          :input-extension="store.inputFile?.extension ?? ''"
        />

        <QualitySlider v-model="store.quality" />

        <ConversionProgress
          :stage="store.stage"
          :percent="store.progress.percent"
          :output-format="store.outputFormat"
          :can-convert="store.canConvert"
          :error-message="store.error"
          @convert="onConvert"
        />
      </div>

      <!-- RIGHT: Output / Result -->
      <div class="convert-page__panel glass-card">
        <div class="panel-header">
          <q-icon name="sym_r_output" size="20px" color="primary" />
          <span>Output</span>
        </div>

        <template v-if="store.result">
          <ResultCard
            :result="store.result"
            @convert-another="store.clearInputFile()"
          />
        </template>

        <template v-else-if="store.stage === 'idle'">
          <div class="output-preview">
            <q-icon name="sym_r_arrow_forward" size="40px" class="text-glass" />
            <div class="text-glass" style="font-size: 14px; text-align: center">
              Select a format and click convert
            </div>
          </div>
        </template>

        <template v-else-if="store.stage === 'error'">
          <div class="output-preview">
            <q-icon name="sym_r_error" size="40px" color="negative" />
            <div style="font-size: 13px; color: var(--q-negative); font-weight: 600">
              Conversion failed
            </div>
            <div class="text-glass mono" style="font-size: 12px; text-align: center; max-width: 260px">
              {{ store.error || 'Unknown error' }}
            </div>
          </div>
        </template>

        <template v-else>
          <div class="output-preview">
            <q-spinner-dots color="primary" size="40px" />
            <div class="text-glass" style="font-size: 13px">
              {{ stageMessage }}
            </div>
          </div>
        </template>
      </div>
    </div>

    <q-dialog v-model="overwriteDialogOpen" persistent>
      <q-card class="overwrite-dialog glass-card">
        <q-card-section>
          <div class="overwrite-dialog__title">Output file already exists</div>
          <div class="overwrite-dialog__text">
            A file with this name already exists. Do you want to overwrite it?
          </div>
          <div class="overwrite-dialog__path mono">{{ pendingOutputPath }}</div>
        </q-card-section>

        <q-card-section class="q-pt-none">
          <q-checkbox
            v-model="overwriteByDefault"
            color="primary"
            label="Overwrite files by default"
          />
        </q-card-section>

        <q-card-actions align="right">
          <q-btn flat no-caps label="Cancel" color="grey-5" @click="cancelOverwrite" />
          <q-btn unelevated no-caps label="Overwrite" color="negative" @click="confirmOverwrite" />
        </q-card-actions>
      </q-card>
    </q-dialog>
  </q-page>
</template>

<script setup lang="ts">
import ConversionProgress from 'src/components/ConversionProgress.vue';
import FileDropZone from 'src/components/FileDropZone.vue';
import FormatSelector from 'src/components/FormatSelector.vue';
import QualitySlider from 'src/components/QualitySlider.vue';
import ResultCard from 'src/components/ResultCard.vue';
import { useConvert } from 'src/composables/useConvert';
import { useConversionStore } from 'src/stores/conversion';
import { useSettingsStore } from 'src/stores/settings';
import type { FileInfo } from 'src/types/conversion';
import { getFormatCategory } from 'src/types/formats';
import { computed, ref } from 'vue';

const store = useConversionStore();
const settings = useSettingsStore();
const { convert, getPlannedOutputPath, pathExists } = useConvert();

const overwriteDialogOpen = ref(false);
const overwriteByDefault = ref(false);
const pendingOutputPath = ref('');

function onFileSelected(info: FileInfo) {
  store.setInputFile(info);
}

async function onConvert() {
  const outputPath = getPlannedOutputPath();

  if (outputPath && !settings.overwriteExisting) {
    try {
      const exists = await pathExists(outputPath);
      if (exists) {
        pendingOutputPath.value = outputPath;
        overwriteByDefault.value = settings.overwriteExisting;
        overwriteDialogOpen.value = true;
        return;
      }
    } catch {
      // Fall back to backend error handling below if pre-check is unavailable.
    }
  }

  const outcome = await convert();
  if (!settings.overwriteExisting && !outcome.ok && outcome.reason === 'output_exists') {
    pendingOutputPath.value = outputPath || '';
    overwriteByDefault.value = settings.overwriteExisting;
    overwriteDialogOpen.value = true;
  }
}

function cancelOverwrite() {
  overwriteDialogOpen.value = false;
}

async function confirmOverwrite() {
  if (overwriteByDefault.value !== settings.overwriteExisting) {
    settings.overwriteExisting = overwriteByDefault.value;
  }
  overwriteDialogOpen.value = false;
  await convert(true);
}

const categoryIcon = computed(() => {
  const cat = getFormatCategory(store.inputFile?.extension ?? '');
  if (cat === 'image') return 'sym_r_image';
  if (cat === 'video') return 'sym_r_movie';
  if (cat === 'audio') return 'sym_r_audio_file';
  return 'sym_r_description';
});

const stageMessage = computed(() => {
  switch (store.stage) {
    case 'reading': return 'Reading input file...';
    case 'converting': return 'Converting...';
    case 'writing': return 'Writing output...';
    default: return '';
  }
});

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}
</script>

<style lang="scss" scoped>
.convert-page {
  position: relative;
  z-index: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: calc(100vh - 40px);
  padding: 24px 32px;

  &__panels {
    display: grid;
    grid-template-columns: 1fr 1.4fr 1fr;
    gap: 20px;
    width: 100%;
    max-width: 1100px;
    animation: fade-in 0.3s ease forwards;
  }

  &__panel {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 24px;

    &--center {
      gap: 24px;
    }
  }
}

.panel-header {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  font-weight: 600;
  color: rgba(255, 255, 255, 0.5);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.file-info {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
  padding: 20px 0;

  &__icon {
    color: rgba(255, 255, 255, 0.2);
  }

  &__name {
    font-size: 14px;
    font-weight: 500;
    color: rgba(255, 255, 255, 0.8);
    word-break: break-all;
    text-align: center;
  }

  &__meta {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
  }

  &__badge {
    padding: 2px 8px;
    border-radius: 6px;
    font-size: 11px;
    font-weight: 600;
    background: rgba($primary, 0.15);
    color: $primary;
  }
}

.output-preview {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  flex: 1;
  min-height: 200px;
}

.overwrite-dialog {
  width: min(560px, 92vw);
  border-radius: 14px;

  &__title {
    font-size: 16px;
    font-weight: 700;
    color: rgba(255, 255, 255, 0.9);
    margin-bottom: 8px;
  }

  &__text {
    font-size: 13px;
    color: rgba(255, 255, 255, 0.7);
    margin-bottom: 10px;
  }

  &__path {
    font-size: 11px;
    line-height: 1.4;
    word-break: break-all;
    color: rgba(255, 255, 255, 0.55);
  }
}
</style>
