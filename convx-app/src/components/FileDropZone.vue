<template>
  <div
    class="dropzone"
    :class="{ 'dropzone--over': isDragOver, 'dropzone--has-file': hasFile }"
    role="button"
    tabindex="0"
    aria-label="Drop files here or click to browse. Supports 53 formats."
    @dragover.prevent="onDragOver"
    @dragleave.prevent="onDragLeave"
    @drop.prevent="onDrop"
    @click="openPicker"
    @keydown.enter="openPicker"
    @keydown.space.prevent="openPicker"
  >
    <input
      ref="fileInputRef"
      type="file"
      class="dropzone__input"
      :accept="acceptedFormats"
      @change="onFileSelect"
    />

    <div class="dropzone__content">
      <q-icon
        :name="isDragOver ? 'sym_r_download' : 'sym_r_cloud_upload'"
        :size="isDragOver ? '64px' : '56px'"
        :color="isDragOver ? 'primary' : undefined"
        class="dropzone__icon"
      />
      <div class="dropzone__title">
        {{ isDragOver ? 'Drop to convert' : 'Drop files here' }}
      </div>
      <div class="dropzone__subtitle">or click to browse</div>
      <div class="dropzone__stats">53 formats · 400+ conversions</div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue';
import { isTauri } from '../services/bridge';
import type { FileInfo } from '../types/conversion';

const emit = defineEmits<{
  fileSelected: [info: FileInfo];
}>();

const fileInputRef = ref<HTMLInputElement>();
const isDragOver = ref(false);
const hasFile = ref(false);

const acceptedFormats = [
  'image/*',
  'video/*',
  'audio/*',
  'application/pdf',
  '.doc,.docx,.pptx,.xlsx,.txt,.md,.html,.csv,.json,.yaml,.yml,.xml,.epub,.mobi',
].join(',');

function onDragOver() {
  isDragOver.value = true;
}

function onDragLeave() {
  isDragOver.value = false;
}

function onDrop(event: DragEvent) {
  isDragOver.value = false;
  const files = event.dataTransfer?.files;
  if (files && files.length > 0) {
    handleFile(files[0] as File);
  }
}

async function openPicker() {
  if (isTauri()) {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({
        multiple: false,
        filters: [
          { name: 'All Supported', extensions: [
            'png', 'jpg', 'jpeg', 'webp', 'gif', 'bmp', 'tiff', 'ico', 'svg', 'avif', 'heic', 'heif',
            'mp4', 'mov', 'webm', 'avi', 'mkv', 'wmv', 'flv', 'm4v', 'mpeg', 'ts',
            'mp3', 'wav', 'flac', 'm4a', 'aac', 'ogg', 'wma', 'aiff', 'opus', 'ac3',
            'pdf', 'doc', 'docx', 'pptx', 'xlsx', 'txt', 'md', 'html',
            'csv', 'json', 'yaml', 'yml', 'xml',
            'epub', 'mobi',
          ] },
        ],
      });
      if (selected && typeof selected === 'string') {
        // In Tauri we get a file path string
        const { getBridge } = await import('../services/bridge');
        const bridge = await getBridge();
        const info = await bridge.getFileInfo(selected);
        hasFile.value = true;
        emit('fileSelected', {
          path: selected,
          name: info.name,
          extension: info.extension,
          size: info.size,
        });
      }
    } catch (e) {
      console.error('File dialog error:', e);
    }
  } else {
    fileInputRef.value?.click();
  }
}

function onFileSelect(event: Event) {
  const input = event.target as HTMLInputElement;
  if (input.files && input.files.length > 0) {
    handleFile(input.files[0] as File);
  }
}

function handleFile(file: File) {
  const name = file.name;
  const extension = name.split('.').pop()?.toLowerCase() || '';
  hasFile.value = true;
  emit('fileSelected', {
    path: (file as unknown as { path?: string }).path || name,
    name,
    extension,
    size: file.size,
  });
}
</script>

<style lang="scss" scoped>
.dropzone {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 320px;
  max-width: 560px;
  margin: 0 auto;
  border: 2px dashed rgba(255, 255, 255, 0.1);
  border-radius: 20px;
  padding: 48px 40px;
  cursor: pointer;
  transition: all 0.3s ease;
  overflow: hidden;

  &::before {
    content: '';
    position: absolute;
    inset: 0;
    border-radius: 20px;
    opacity: 0;
    transition: opacity 0.3s ease;
    background: radial-gradient(
      600px circle at var(--mouse-x, 50%) var(--mouse-y, 50%),
      rgba($primary, 0.06),
      transparent 70%
    );
  }

  &:hover {
    border-color: rgba(255, 255, 255, 0.15);
    &::before {
      opacity: 1;
    }
  }

  &--over {
    border-color: $primary;
    border-style: solid;
    animation: pulse-glow 1.5s ease-in-out infinite;

    &::before {
      opacity: 1;
      background: radial-gradient(
        600px circle at 50% 50%,
        rgba($primary, 0.12),
        transparent 70%
      );
    }
  }

  &__input {
    display: none;
  }

  &__content {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    z-index: 1;
  }

  &__icon {
    color: rgba(255, 255, 255, 0.45);
    transition: all 0.3s ease;

    .dropzone--over & {
      transform: scale(1.1);
    }
  }

  &__title {
    font-size: 20px;
    font-weight: 600;
    color: rgba(255, 255, 255, 0.7);
    letter-spacing: -0.3px;
  }

  &__subtitle {
    font-size: 14px;
    color: rgba(255, 255, 255, 0.65);
  }

  &__stats {
    margin-top: 4px;
    font-size: 13px;
    font-weight: 500;
    letter-spacing: 0.3px;
    color: rgba(255, 255, 255, 0.5);
  }
}
</style>
