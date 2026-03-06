# convx Frontend Specification (Quasar)

**Version:** 0.1.0  
**Framework:** Quasar (Vue 3)  
**Platforms:** Desktop (Tauri), Web (SPA), Mobile (Capacitor)

---

## Overview

One Quasar codebase → four platforms:

```
┌─────────────────────────────────────────────────────┐
│              Quasar App (Vue 3 + TypeScript)        │
│                   Single Codebase                   │
└─────────────────────────┬───────────────────────────┘
                          │
        ┌─────────────────┼─────────────────┬─────────────────┐
        │                 │                 │                 │
        ▼                 ▼                 ▼                 ▼
   ┌─────────┐      ┌─────────┐      ┌─────────┐      ┌─────────┐
   │   Web   │      │ Desktop │      │   iOS   │      │ Android │
   │  (SPA)  │      │ (Tauri) │      │(Capacitor)     │(Capacitor)
   └────┬────┘      └────┬────┘      └────┬────┘      └────┬────┘
        │                │                │                │
        ▼                ▼                └───────┬────────┘
   ┌─────────┐      ┌─────────┐              ┌────▼────┐
   │  WASM   │      │  Rust   │              │   FFI   │
   │ Bridge  │      │  IPC    │              │ Bridge  │
   └────┬────┘      └────┬────┘              └────┬────┘
        │                │                        │
        └────────────────┴────────────────────────┘
                         │
              ┌──────────▼──────────┐
              │    convx-core       │
              │    (Rust Engine)    │
              └─────────────────────┘
```

---

## Project Structure

```
convx-app/
├── quasar.config.js           # Quasar configuration
├── package.json
├── tsconfig.json
├── src/
│   ├── App.vue
│   ├── router/
│   │   └── routes.ts
│   ├── stores/                # Pinia stores
│   │   ├── conversion.ts      # Conversion state
│   │   ├── settings.ts        # User preferences
│   │   └── history.ts         # Conversion history
│   ├── composables/           # Vue composables
│   │   ├── useConvert.ts      # Conversion logic
│   │   ├── usePlatform.ts     # Platform detection
│   │   └── useRustBridge.ts   # Rust communication
│   ├── components/
│   │   ├── FileDropZone.vue
│   │   ├── FormatSelector.vue
│   │   ├── QualitySlider.vue
│   │   ├── ConversionProgress.vue
│   │   ├── PresetPicker.vue
│   │   ├── BatchList.vue
│   │   └── ResultCard.vue
│   ├── pages/
│   │   ├── IndexPage.vue      # Main conversion UI
│   │   ├── BatchPage.vue      # Batch processing
│   │   ├── SettingsPage.vue   # Preferences
│   │   └── HistoryPage.vue    # Past conversions
│   ├── layouts/
│   │   └── MainLayout.vue
│   ├── services/
│   │   ├── bridge/
│   │   │   ├── index.ts       # Unified bridge API
│   │   │   ├── tauri.ts       # Tauri IPC
│   │   │   ├── wasm.ts        # WASM bindings
│   │   │   └── capacitor.ts   # Mobile FFI
│   │   ├── fileUtils.ts
│   │   └── formatUtils.ts
│   ├── types/
│   │   ├── conversion.ts
│   │   └── formats.ts
│   └── css/
│       └── app.scss
├── src-tauri/                 # Tauri backend
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── src/
│       ├── main.rs
│       └── commands.rs        # IPC handlers
├── src-capacitor/             # Capacitor config
│   └── capacitor.config.ts
└── public/
    └── icons/
```

---

## Quasar Configuration

**File:** `quasar.config.js`

```javascript
const { configure } = require('quasar/wrappers')

module.exports = configure(function (ctx) {
  return {
    boot: ['bridge'],
    
    css: ['app.scss'],
    
    extras: [
      'roboto-font',
      'material-icons',
    ],
    
    build: {
      target: {
        browser: ['es2020', 'chrome100', 'safari15'],
        node: 'node18'
      },
      vueRouterMode: 'history',
      typescript: {
        strict: true
      },
      env: {
        PLATFORM: ctx.modeName // spa, tauri, capacitor
      }
    },
    
    devServer: {
      open: true
    },
    
    framework: {
      config: {
        dark: 'auto',
        notify: { position: 'top-right' }
      },
      plugins: ['Notify', 'Dialog', 'Loading']
    },
    
    // Desktop
    tauri: {
      bundle: {
        active: true,
        targets: ['app', 'dmg', 'msi'],
        identifier: 'com.convx.app',
        icon: 'src-tauri/icons/icon.icns'
      }
    },
    
    // Mobile
    capacitor: {
      hideSplashscreen: true,
      capacitorCliPreparationParams: ['sync']
    }
  }
})
```

---

## Rust Bridge Layer

### Unified API

**File:** `src/services/bridge/index.ts`

```typescript
import { Platform } from 'quasar'
import { TauriBridge } from './tauri'
import { WasmBridge } from './wasm'
import { CapacitorBridge } from './capacitor'

export interface ConversionOptions {
  outputFormat: string
  quality?: number
  width?: number
  height?: number
  preset?: string
}

export interface ConversionResult {
  id: string
  status: 'completed' | 'failed'
  inputPath: string
  outputPath?: string
  inputSize: number
  outputSize?: number
  spaceSaved?: number
  durationMs: number
  error?: string
}

export interface FileInfo {
  path: string
  format: string
  size: number
  metadata: Record<string, unknown>
}

export interface ConvxBridge {
  // Core conversion
  convert(input: string | Uint8Array, output: string, options: ConversionOptions): Promise<ConversionResult>
  
  // File info
  getFileInfo(path: string): Promise<FileInfo>
  
  // Format support
  canConvert(from: string, to: string): Promise<boolean>
  getSupportedFormats(): Promise<string[]>
  getConversionTargets(from: string): Promise<string[]>
  
  // Presets
  getPresets(): Promise<Preset[]>
  
  // Batch
  convertBatch(inputs: string[], outputDir: string, options: ConversionOptions): Promise<ConversionResult[]>
  
  // Progress callback
  onProgress(callback: (progress: ProgressEvent) => void): void
}

// Factory function - returns correct bridge for platform
export function createBridge(): ConvxBridge {
  if (process.env.PLATFORM === 'tauri') {
    return new TauriBridge()
  } else if (Platform.is.capacitor) {
    return new CapacitorBridge()
  } else {
    return new WasmBridge()
  }
}

// Singleton instance
let bridge: ConvxBridge | null = null

export function useBridge(): ConvxBridge {
  if (!bridge) {
    bridge = createBridge()
  }
  return bridge
}
```

### Tauri Bridge

**File:** `src/services/bridge/tauri.ts`

```typescript
import { invoke } from '@tauri-apps/api/tauri'
import { listen } from '@tauri-apps/api/event'
import type { ConvxBridge, ConversionOptions, ConversionResult, FileInfo, Preset } from './index'

export class TauriBridge implements ConvxBridge {
  private progressCallback?: (progress: ProgressEvent) => void
  
  constructor() {
    // Listen for progress events from Rust
    listen<ProgressEvent>('conversion-progress', (event) => {
      if (this.progressCallback) {
        this.progressCallback(event.payload)
      }
    })
  }
  
  async convert(input: string, output: string, options: ConversionOptions): Promise<ConversionResult> {
    return invoke('convert_file', {
      input,
      output,
      options
    })
  }
  
  async getFileInfo(path: string): Promise<FileInfo> {
    return invoke('get_file_info', { path })
  }
  
  async canConvert(from: string, to: string): Promise<boolean> {
    return invoke('can_convert', { from, to })
  }
  
  async getSupportedFormats(): Promise<string[]> {
    return invoke('get_supported_formats')
  }
  
  async getConversionTargets(from: string): Promise<string[]> {
    return invoke('get_conversion_targets', { from })
  }
  
  async getPresets(): Promise<Preset[]> {
    return invoke('get_presets')
  }
  
  async convertBatch(inputs: string[], outputDir: string, options: ConversionOptions): Promise<ConversionResult[]> {
    return invoke('convert_batch', {
      inputs,
      outputDir,
      options
    })
  }
  
  onProgress(callback: (progress: ProgressEvent) => void): void {
    this.progressCallback = callback
  }
}
```

### WASM Bridge

**File:** `src/services/bridge/wasm.ts`

```typescript
import init, * as wasm from 'convx-wasm'
import type { ConvxBridge, ConversionOptions, ConversionResult, FileInfo, Preset } from './index'

let initialized = false

export class WasmBridge implements ConvxBridge {
  private progressCallback?: (progress: ProgressEvent) => void
  
  private async ensureInit() {
    if (!initialized) {
      await init()
      initialized = true
    }
  }
  
  async convert(input: string | Uint8Array, output: string, options: ConversionOptions): Promise<ConversionResult> {
    await this.ensureInit()
    
    // For web, input is always bytes
    const inputBytes = input instanceof Uint8Array ? input : new TextEncoder().encode(input)
    const inputFormat = this.detectFormat(output) // Detect from desired output
    
    const outputBytes = wasm.convert(
      inputBytes,
      options.outputFormat,
      JSON.stringify(options)
    )
    
    // Trigger download
    this.downloadBlob(outputBytes, output)
    
    return {
      id: crypto.randomUUID(),
      status: 'completed',
      inputPath: 'memory',
      outputPath: output,
      inputSize: inputBytes.length,
      outputSize: outputBytes.length,
      spaceSaved: inputBytes.length - outputBytes.length,
      durationMs: 0 // TODO: measure
    }
  }
  
  private downloadBlob(bytes: Uint8Array, filename: string) {
    const blob = new Blob([bytes])
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = filename
    a.click()
    URL.revokeObjectURL(url)
  }
  
  private detectFormat(filename: string): string {
    const ext = filename.split('.').pop()?.toLowerCase() || ''
    return ext
  }
  
  async getFileInfo(path: string): Promise<FileInfo> {
    await this.ensureInit()
    // For web, we'd need the file bytes
    throw new Error('getFileInfo requires file bytes on web')
  }
  
  async canConvert(from: string, to: string): Promise<boolean> {
    await this.ensureInit()
    return wasm.can_convert(from, to)
  }
  
  async getSupportedFormats(): Promise<string[]> {
    await this.ensureInit()
    return JSON.parse(wasm.get_supported_formats())
  }
  
  async getConversionTargets(from: string): Promise<string[]> {
    await this.ensureInit()
    return JSON.parse(wasm.get_conversion_targets(from))
  }
  
  async getPresets(): Promise<Preset[]> {
    await this.ensureInit()
    return JSON.parse(wasm.get_presets())
  }
  
  async convertBatch(inputs: string[], outputDir: string, options: ConversionOptions): Promise<ConversionResult[]> {
    // Web doesn't support batch with file paths
    throw new Error('Batch conversion not supported on web. Use desktop app.')
  }
  
  onProgress(callback: (progress: ProgressEvent) => void): void {
    this.progressCallback = callback
  }
}
```

### Capacitor Bridge

**File:** `src/services/bridge/capacitor.ts`

```typescript
import { registerPlugin } from '@capacitor/core'
import type { ConvxBridge, ConversionOptions, ConversionResult, FileInfo, Preset } from './index'

interface ConvxPlugin {
  convert(options: { input: string; output: string; options: string }): Promise<ConversionResult>
  getFileInfo(options: { path: string }): Promise<FileInfo>
  canConvert(options: { from: string; to: string }): Promise<{ result: boolean }>
  getSupportedFormats(): Promise<{ formats: string[] }>
  getConversionTargets(options: { from: string }): Promise<{ targets: string[] }>
  getPresets(): Promise<{ presets: Preset[] }>
}

const ConvxNative = registerPlugin<ConvxPlugin>('Convx')

export class CapacitorBridge implements ConvxBridge {
  private progressCallback?: (progress: ProgressEvent) => void
  
  async convert(input: string, output: string, options: ConversionOptions): Promise<ConversionResult> {
    return ConvxNative.convert({
      input,
      output,
      options: JSON.stringify(options)
    })
  }
  
  async getFileInfo(path: string): Promise<FileInfo> {
    return ConvxNative.getFileInfo({ path })
  }
  
  async canConvert(from: string, to: string): Promise<boolean> {
    const { result } = await ConvxNative.canConvert({ from, to })
    return result
  }
  
  async getSupportedFormats(): Promise<string[]> {
    const { formats } = await ConvxNative.getSupportedFormats()
    return formats
  }
  
  async getConversionTargets(from: string): Promise<string[]> {
    const { targets } = await ConvxNative.getConversionTargets({ from })
    return targets
  }
  
  async getPresets(): Promise<Preset[]> {
    const { presets } = await ConvxNative.getPresets()
    return presets
  }
  
  async convertBatch(inputs: string[], outputDir: string, options: ConversionOptions): Promise<ConversionResult[]> {
    // Convert one at a time on mobile
    const results: ConversionResult[] = []
    for (const input of inputs) {
      const filename = input.split('/').pop() || 'output'
      const output = `${outputDir}/${filename}.${options.outputFormat}`
      const result = await this.convert(input, output, options)
      results.push(result)
    }
    return results
  }
  
  onProgress(callback: (progress: ProgressEvent) => void): void {
    this.progressCallback = callback
  }
}
```

---

## Vue Components

### Main Conversion Page

**File:** `src/pages/IndexPage.vue`

```vue
<template>
  <q-page class="q-pa-md">
    <div class="row q-col-gutter-md">
      <!-- Drop Zone -->
      <div class="col-12">
        <FileDropZone 
          v-model="files"
          @files-dropped="onFilesDropped"
        />
      </div>
      
      <!-- Conversion Options -->
      <div class="col-12 col-md-6" v-if="files.length">
        <q-card>
          <q-card-section>
            <div class="text-h6">Output Format</div>
            <FormatSelector 
              v-model="outputFormat"
              :input-format="detectedFormat"
            />
          </q-card-section>
          
          <q-card-section>
            <div class="text-h6">Quality</div>
            <QualitySlider v-model="quality" />
          </q-card-section>
          
          <q-card-section>
            <PresetPicker 
              v-model="selectedPreset"
              :input-format="detectedFormat"
            />
          </q-card-section>
        </q-card>
      </div>
      
      <!-- Preview / Result -->
      <div class="col-12 col-md-6" v-if="files.length">
        <q-card>
          <q-card-section v-if="!converting && !result">
            <div class="text-h6">Preview</div>
            <div class="text-body2 text-grey">
              {{ files[0].name }} ({{ formatSize(files[0].size) }})
            </div>
          </q-card-section>
          
          <q-card-section v-if="converting">
            <ConversionProgress :progress="progress" />
          </q-card-section>
          
          <q-card-section v-if="result">
            <ResultCard :result="result" />
          </q-card-section>
        </q-card>
      </div>
      
      <!-- Convert Button -->
      <div class="col-12" v-if="files.length">
        <q-btn
          color="primary"
          size="lg"
          :loading="converting"
          :disable="!canConvert"
          @click="convert"
          class="full-width"
        >
          <q-icon left name="transform" />
          Convert to {{ outputFormat.toUpperCase() }}
        </q-btn>
      </div>
    </div>
  </q-page>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useBridge } from 'src/services/bridge'
import { useConversionStore } from 'src/stores/conversion'
import FileDropZone from 'src/components/FileDropZone.vue'
import FormatSelector from 'src/components/FormatSelector.vue'
import QualitySlider from 'src/components/QualitySlider.vue'
import PresetPicker from 'src/components/PresetPicker.vue'
import ConversionProgress from 'src/components/ConversionProgress.vue'
import ResultCard from 'src/components/ResultCard.vue'

const bridge = useBridge()
const store = useConversionStore()

const files = ref<File[]>([])
const outputFormat = ref('webp')
const quality = ref(80)
const selectedPreset = ref<string | null>(null)
const converting = ref(false)
const progress = ref(0)
const result = ref<ConversionResult | null>(null)

const detectedFormat = computed(() => {
  if (!files.value.length) return null
  const ext = files.value[0].name.split('.').pop()?.toLowerCase()
  return ext || null
})

const canConvert = computed(() => {
  return files.value.length > 0 && outputFormat.value
})

function onFilesDropped(droppedFiles: File[]) {
  files.value = droppedFiles
  result.value = null
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}

async function convert() {
  if (!files.value.length) return
  
  converting.value = true
  progress.value = 0
  result.value = null
  
  try {
    bridge.onProgress((event) => {
      progress.value = event.percent * 100
    })
    
    const file = files.value[0]
    const outputName = file.name.replace(/\.[^.]+$/, `.${outputFormat.value}`)
    
    // Read file as bytes for WASM, or use path for Tauri/Capacitor
    const input = await getFileInput(file)
    
    result.value = await bridge.convert(input, outputName, {
      outputFormat: outputFormat.value,
      quality: quality.value,
      preset: selectedPreset.value || undefined
    })
    
    // Save to history
    store.addToHistory(result.value)
    
  } catch (error) {
    console.error('Conversion failed:', error)
    // Show error notification
  } finally {
    converting.value = false
  }
}

async function getFileInput(file: File): Promise<string | Uint8Array> {
  // For web (WASM), return bytes
  if (process.env.PLATFORM === 'spa') {
    const buffer = await file.arrayBuffer()
    return new Uint8Array(buffer)
  }
  
  // For desktop/mobile, return path
  // (Tauri and Capacitor handle file paths differently)
  return (file as any).path || file.name
}
</script>
```

### File Drop Zone Component

**File:** `src/components/FileDropZone.vue`

```vue
<template>
  <div
    class="drop-zone"
    :class="{ 'drag-over': isDragOver, 'has-files': modelValue.length }"
    @dragover.prevent="isDragOver = true"
    @dragleave.prevent="isDragOver = false"
    @drop.prevent="onDrop"
    @click="openFilePicker"
  >
    <input
      ref="fileInput"
      type="file"
      multiple
      :accept="acceptedFormats"
      class="hidden"
      @change="onFileSelect"
    />
    
    <div class="drop-zone-content">
      <q-icon 
        :name="isDragOver ? 'file_download' : 'cloud_upload'" 
        size="64px"
        :color="isDragOver ? 'primary' : 'grey-6'"
      />
      <div class="text-h6 q-mt-md">
        {{ isDragOver ? 'Drop files here' : 'Drag & drop files' }}
      </div>
      <div class="text-body2 text-grey">
        or click to browse
      </div>
      <div class="text-caption text-grey q-mt-sm">
        Supports: Images, Video, Audio, Documents, Data, Ebooks
      </div>
    </div>
    
    <!-- File list preview -->
    <div v-if="modelValue.length" class="file-list q-mt-md">
      <q-chip
        v-for="(file, index) in modelValue"
        :key="index"
        removable
        @remove="removeFile(index)"
        color="primary"
        text-color="white"
      >
        {{ file.name }}
      </q-chip>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'

const props = defineProps<{
  modelValue: File[]
}>()

const emit = defineEmits<{
  'update:modelValue': [files: File[]]
  'files-dropped': [files: File[]]
}>()

const fileInput = ref<HTMLInputElement>()
const isDragOver = ref(false)

const acceptedFormats = [
  'image/*',
  'video/*',
  'audio/*',
  'application/pdf',
  '.doc,.docx,.txt,.md,.html',
  '.csv,.json,.yaml,.yml,.xml,.tsv,.jsonl,.parquet,.arrow,.sqlite,.npy,.npz,.h5',
  '.epub,.mobi'
].join(',')

function openFilePicker() {
  fileInput.value?.click()
}

function onFileSelect(event: Event) {
  const input = event.target as HTMLInputElement
  if (input.files) {
    const files = Array.from(input.files)
    emit('update:modelValue', files)
    emit('files-dropped', files)
  }
}

function onDrop(event: DragEvent) {
  isDragOver.value = false
  const files = Array.from(event.dataTransfer?.files || [])
  if (files.length) {
    emit('update:modelValue', files)
    emit('files-dropped', files)
  }
}

function removeFile(index: number) {
  const files = [...props.modelValue]
  files.splice(index, 1)
  emit('update:modelValue', files)
}
</script>

<style lang="scss" scoped>
.drop-zone {
  border: 2px dashed $grey-4;
  border-radius: 12px;
  padding: 48px 24px;
  text-align: center;
  cursor: pointer;
  transition: all 0.2s ease;
  
  &:hover {
    border-color: $primary;
    background: rgba($primary, 0.05);
  }
  
  &.drag-over {
    border-color: $primary;
    background: rgba($primary, 0.1);
    transform: scale(1.01);
  }
  
  &.has-files {
    border-style: solid;
  }
}

.hidden {
  display: none;
}
</style>
```

### Format Selector Component

**File:** `src/components/FormatSelector.vue`

```vue
<template>
  <div class="format-selector">
    <q-btn-toggle
      v-model="selectedCategory"
      spread
      no-caps
      toggle-color="primary"
      :options="categories"
      class="q-mb-md"
    />
    
    <div class="format-grid">
      <q-btn
        v-for="format in availableFormats"
        :key="format"
        :outline="modelValue !== format"
        :color="modelValue === format ? 'primary' : 'grey-7'"
        :label="format.toUpperCase()"
        no-caps
        @click="emit('update:modelValue', format)"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'
import { useBridge } from 'src/services/bridge'

const props = defineProps<{
  modelValue: string
  inputFormat: string | null
}>()

const emit = defineEmits<{
  'update:modelValue': [format: string]
}>()

const bridge = useBridge()
const selectedCategory = ref('image')
const conversionTargets = ref<string[]>([])

const categories = [
  { label: 'Image', value: 'image' },
  { label: 'Video', value: 'video' },
  { label: 'Audio', value: 'audio' },
  { label: 'Document', value: 'document' },
  { label: 'Data', value: 'data' },
  { label: 'Ebook', value: 'ebook' },
]

const formatsByCategory: Record<string, string[]> = {
  image: ['png', 'jpg', 'webp', 'gif', 'bmp', 'tiff', 'avif', 'heic'],
  video: ['mp4', 'webm', 'mov', 'avi', 'mkv', 'gif'],
  audio: ['mp3', 'wav', 'flac', 'm4a', 'aac', 'ogg', 'opus'],
  document: ['pdf', 'docx', 'txt', 'md', 'html'],
  data: ['csv', 'json', 'yaml', 'xml', 'parquet', 'jsonl', 'tsv', 'arrow', 'sqlite', 'npy', 'npz', 'h5'],
  ebook: ['epub', 'mobi'],
}

const availableFormats = computed(() => {
  const categoryFormats = formatsByCategory[selectedCategory.value] || []
  
  // Filter to only formats we can actually convert to
  if (conversionTargets.value.length) {
    return categoryFormats.filter(f => conversionTargets.value.includes(f))
  }
  
  return categoryFormats
})

// Load conversion targets when input format changes
watch(() => props.inputFormat, async (format) => {
  if (format) {
    conversionTargets.value = await bridge.getConversionTargets(format)
  }
}, { immediate: true })
</script>

<style lang="scss" scoped>
.format-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(70px, 1fr));
  gap: 8px;
}
</style>
```

### Conversion Progress Component

**File:** `src/components/ConversionProgress.vue`

```vue
<template>
  <div class="conversion-progress">
    <q-circular-progress
      :value="progress"
      size="120px"
      :thickness="0.2"
      color="primary"
      track-color="grey-3"
      show-value
      class="q-mb-md"
    >
      <span class="text-h5">{{ Math.round(progress) }}%</span>
    </q-circular-progress>
    
    <div class="text-body1">Converting...</div>
    <div class="text-caption text-grey" v-if="stage">{{ stage }}</div>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  progress: number
  stage?: string
}>()
</script>

<style lang="scss" scoped>
.conversion-progress {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 24px;
}
</style>
```

### Result Card Component

**File:** `src/components/ResultCard.vue`

```vue
<template>
  <div class="result-card">
    <q-icon 
      :name="result.status === 'completed' ? 'check_circle' : 'error'" 
      :color="result.status === 'completed' ? 'positive' : 'negative'"
      size="48px"
    />
    
    <div class="text-h6 q-mt-md">
      {{ result.status === 'completed' ? 'Conversion Complete!' : 'Conversion Failed' }}
    </div>
    
    <template v-if="result.status === 'completed'">
      <div class="stats q-mt-md">
        <div class="stat">
          <div class="stat-label">Original</div>
          <div class="stat-value">{{ formatSize(result.inputSize) }}</div>
        </div>
        <q-icon name="arrow_forward" color="grey" />
        <div class="stat">
          <div class="stat-label">New</div>
          <div class="stat-value">{{ formatSize(result.outputSize || 0) }}</div>
        </div>
      </div>
      
      <q-chip 
        v-if="result.spaceSaved && result.spaceSaved > 0"
        color="positive" 
        text-color="white"
        icon="trending_down"
      >
        Saved {{ formatSize(result.spaceSaved) }} ({{ savingsPercent }}%)
      </q-chip>
      
      <div class="text-caption text-grey q-mt-sm">
        Completed in {{ result.durationMs }}ms
      </div>
    </template>
    
    <template v-else>
      <div class="text-body2 text-negative q-mt-md">
        {{ result.error }}
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { ConversionResult } from 'src/services/bridge'

const props = defineProps<{
  result: ConversionResult
}>()

const savingsPercent = computed(() => {
  if (!props.result.spaceSaved || !props.result.inputSize) return 0
  return Math.round((props.result.spaceSaved / props.result.inputSize) * 100)
})

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}
</script>

<style lang="scss" scoped>
.result-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 24px;
  text-align: center;
}

.stats {
  display: flex;
  align-items: center;
  gap: 16px;
}

.stat {
  text-align: center;
  
  .stat-label {
    font-size: 12px;
    color: $grey-6;
  }
  
  .stat-value {
    font-size: 18px;
    font-weight: 600;
  }
}
</style>
```

---

## Pinia Store

**File:** `src/stores/conversion.ts`

```typescript
import { defineStore } from 'pinia'
import type { ConversionResult } from 'src/services/bridge'

interface ConversionState {
  history: ConversionResult[]
  maxHistory: number
}

export const useConversionStore = defineStore('conversion', {
  state: (): ConversionState => ({
    history: [],
    maxHistory: 100
  }),
  
  getters: {
    recentConversions: (state) => state.history.slice(0, 10),
    
    totalSpaceSaved: (state) => {
      return state.history.reduce((sum, r) => sum + (r.spaceSaved || 0), 0)
    },
    
    totalConversions: (state) => state.history.length,
    
    successRate: (state) => {
      if (!state.history.length) return 100
      const successful = state.history.filter(r => r.status === 'completed').length
      return Math.round((successful / state.history.length) * 100)
    }
  },
  
  actions: {
    addToHistory(result: ConversionResult) {
      this.history.unshift(result)
      
      // Trim if over max
      if (this.history.length > this.maxHistory) {
        this.history = this.history.slice(0, this.maxHistory)
      }
      
      // Persist
      this.persist()
    },
    
    clearHistory() {
      this.history = []
      this.persist()
    },
    
    persist() {
      localStorage.setItem('convx-history', JSON.stringify(this.history))
    },
    
    load() {
      const saved = localStorage.getItem('convx-history')
      if (saved) {
        try {
          this.history = JSON.parse(saved)
        } catch {
          this.history = []
        }
      }
    }
  }
})
```

---

## Tauri Backend

**File:** `src-tauri/src/commands.rs`

```rust
use convx::{ConvxEngine, ConversionOptions, Format};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Serialize)]
pub struct ConversionResult {
    id: String,
    status: String,
    input_path: String,
    output_path: Option<String>,
    input_size: u64,
    output_size: Option<u64>,
    space_saved: Option<i64>,
    duration_ms: u64,
    error: Option<String>,
}

#[derive(Deserialize)]
pub struct JsConversionOptions {
    output_format: String,
    quality: Option<u8>,
    width: Option<u32>,
    height: Option<u32>,
    preset: Option<String>,
}

#[tauri::command]
pub async fn convert_file(
    engine: State<'_, ConvxEngine>,
    input: String,
    output: String,
    options: JsConversionOptions,
) -> Result<ConversionResult, String> {
    let output_format = Format::from_extension(&options.output_format)
        .ok_or_else(|| format!("Unknown format: {}", options.output_format))?;
    
    let conv_options = ConversionOptions {
        output_format,
        quality: options.quality,
        ..Default::default()
    };
    
    let result = engine
        .convert(std::path::Path::new(&input), std::path::Path::new(&output), conv_options)
        .map_err(|e| e.to_string())?;
    
    Ok(ConversionResult {
        id: result.id.to_string(),
        status: format!("{:?}", result.status).to_lowercase(),
        input_path: result.input_path.to_string_lossy().to_string(),
        output_path: result.output_path.map(|p| p.to_string_lossy().to_string()),
        input_size: result.input_size,
        output_size: result.output_size,
        space_saved: result.space_saved,
        duration_ms: result.duration_ms,
        error: result.error,
    })
}

#[tauri::command]
pub fn can_convert(
    engine: State<'_, ConvxEngine>,
    from: String,
    to: String,
) -> bool {
    let from_format = Format::from_extension(&from);
    let to_format = Format::from_extension(&to);
    
    match (from_format, to_format) {
        (Some(f), Some(t)) => engine.can_convert(f, t),
        _ => false,
    }
}

#[tauri::command]
pub fn get_supported_formats() -> Vec<String> {
    vec![
        // Images
        "png", "jpg", "webp", "gif", "bmp", "tiff", "ico", "svg", "heic", "heif", "avif",
        // Video
        "mp4", "mov", "webm", "avi", "mkv", "wmv", "flv", "m4v", "mpeg", "ts",
        // Audio
        "mp3", "wav", "flac", "m4a", "aac", "ogg", "wma", "aiff", "opus", "ac3",
        // Documents
        "pdf", "docx", "doc", "pptx", "xlsx", "txt", "md", "html",
        // Data (including ML formats)
        "csv", "json", "yaml", "xml", "parquet", "jsonl", "tsv", "arrow", "sqlite", "npy", "npz", "h5",
        // Ebooks
        "epub", "mobi",
    ].into_iter().map(String::from).collect()
}

#[tauri::command]
pub fn get_conversion_targets(from: String) -> Vec<String> {
    let format = match Format::from_extension(&from) {
        Some(f) => f,
        None => return vec![],
    };
    
    format.convertible_targets()
        .into_iter()
        .map(|f| f.extension().to_string())
        .collect()
}
```

**File:** `src-tauri/src/main.rs`

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

use convx::ConvxEngine;

fn main() {
    let engine = ConvxEngine::new().expect("Failed to initialize convx engine");
    
    tauri::Builder::default()
        .manage(engine)
        .invoke_handler(tauri::generate_handler![
            commands::convert_file,
            commands::can_convert,
            commands::get_supported_formats,
            commands::get_conversion_targets,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

---

## Build Commands

```bash
# Development
quasar dev                    # Web SPA
quasar dev -m tauri           # Desktop (Tauri)
quasar dev -m capacitor -T ios      # iOS
quasar dev -m capacitor -T android  # Android

# Production builds
quasar build                  # Web SPA
quasar build -m tauri         # Desktop installers
quasar build -m capacitor -T ios    # iOS app
quasar build -m capacitor -T android # Android APK
```

---

## Summary

**One Quasar codebase** builds to:

| Platform | Mode | Rust Bridge | Output |
|----------|------|-------------|--------|
| Web | SPA | WASM | Static files |
| Desktop | Tauri | IPC | .dmg, .msi, .AppImage |
| iOS | Capacitor | FFI | .ipa |
| Android | Capacitor | FFI | .apk |

**You write the UI once. Platform differences handled by the bridge layer.**
