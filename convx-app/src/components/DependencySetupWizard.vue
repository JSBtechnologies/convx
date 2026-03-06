<template>
  <q-dialog
    :model-value="modelValue"
    persistent
    maximized
    transition-show="fade"
    transition-hide="fade"
  >
    <q-card class="wizard-card" aria-labelledby="wizard-dialog-title">
      <div class="wizard-content">
        <!-- Installing -->
        <div v-if="stage === 'installing'" class="wizard-center">
          <div id="wizard-dialog-title" class="wizard-title">Verifying setup</div>
          <div class="wizard-message text-glass">{{ loadingMessage }}</div>

          <q-linear-progress
            rounded
            size="8px"
            :value="progress"
            color="primary"
            class="wizard-progress"
          />
          <div class="mono text-glass" style="font-size: 11px">
            {{ currentIndex + 1 }} of {{ totalDeps }}
          </div>

          <div class="wizard-dep-list">
            <div
              v-for="dep in depStates"
              :key="dep.name"
              class="wizard-dep-row"
            >
              <q-icon
                v-if="dep.status === 'done'"
                name="sym_r_check_circle"
                color="positive"
                size="18px"
              />
              <q-icon
                v-else-if="dep.status === 'failed'"
                name="sym_r_error"
                color="negative"
                size="18px"
              />
              <q-spinner-dots
                v-else-if="dep.status === 'installing'"
                size="18px"
                color="primary"
              />
              <div v-else class="wizard-dep-dot" />
              <span :class="dep.status === 'pending' ? 'text-glass' : ''">{{ dep.label }}</span>
            </div>
          </div>
        </div>

        <!-- Ready -->
        <div v-else-if="stage === 'ready'" class="wizard-center">
          <q-icon name="sym_r_check_circle" color="positive" size="56px" />
          <div class="wizard-title">You're all set</div>
          <div class="text-glass">Everything is installed. Start converting files now.</div>
          <q-btn
            color="positive"
            no-caps
            label="Start Converting"
            size="lg"
            style="margin-top: 8px"
            @click="closeWizard"
          />
        </div>

        <!-- Failed -->
        <div v-else-if="stage === 'failed'" class="wizard-manual">
          <div class="wizard-title">Almost there</div>
          <div class="text-glass" style="margin-bottom: 12px">
            Some bundled dependencies could not be verified. You can try reinstalling the app, or install them manually below.
          </div>

          <div class="wizard-dep-list" style="margin-bottom: 16px">
            <div
              v-for="dep in depStates"
              :key="dep.name"
              class="wizard-dep-row"
            >
              <q-icon
                v-if="dep.status === 'done'"
                name="sym_r_check_circle"
                color="positive"
                size="18px"
              />
              <q-icon
                v-else
                name="sym_r_cancel"
                color="negative"
                size="18px"
              />
              <span>{{ dep.label }}</span>
            </div>
          </div>

          <div class="wizard-section glass-card">
            <div class="wizard-section__title">Run this in Terminal</div>
            <div class="wizard-command mono">{{ installCommand }}</div>
            <div class="wizard-actions">
              <q-btn flat color="primary" no-caps icon="sym_r_content_copy" label="Copy" @click="copyCommand" />
            </div>
          </div>

          <div class="wizard-actions" style="margin-top: 12px">
            <q-btn
              color="primary"
              no-caps
              icon="sym_r_refresh"
              label="Re-check"
              :loading="checking"
              @click="recheck"
            />
            <q-btn
              flat
              color="primary"
              no-caps
              label="Retry auto-install"
              :loading="retrying"
              @click="retryInstall"
            />
            <q-btn
              flat
              no-caps
              label="View Docs"
              icon="sym_r_open_in_new"
              style="color: rgba(255,255,255,0.5)"
              @click="openDocs"
            />
          </div>
        </div>
      </div>
    </q-card>
  </q-dialog>
</template>

<script setup lang="ts">
import { open } from '@tauri-apps/plugin-shell';
import { Notify } from 'quasar';
import { computed, onUnmounted, reactive, ref, watch } from 'vue';
import { getBridge } from '../services/bridge';

const props = defineProps<{
  modelValue: boolean;
}>();

const emit = defineEmits<{
  'update:modelValue': [value: boolean];
  ready: [];
}>();

type Stage = 'installing' | 'ready' | 'failed';
type DepStatus = 'pending' | 'installing' | 'done' | 'failed';

interface DepState {
  name: string;
  label: string;
  status: DepStatus;
  error?: string;
}

const FRIENDLY_NAMES: Record<string, string> = {
  ffmpeg: 'FFmpeg',
  vips: 'libvips',
  libreoffice: 'LibreOffice',
  pandoc: 'Pandoc',
  poppler: 'Poppler',
  'python@3': 'Python 3',
  'pip:mobi': 'mobi (Python)',
  'pip:pandas': 'pandas (Python)',
  'pip:openpyxl': 'openpyxl (Python)',
  'pip:weasyprint': 'weasyprint (Python)',
  'pip:pdf2docx': 'pdf2docx (Python)',
  'pip:pyarrow': 'pyarrow (Python)',
  'pip:numpy': 'numpy (Python)',
  'pip:h5py': 'h5py (Python)',
};

const LOADING_MESSAGES = [
  'Verifying bundled dependencies...',
  'Checking conversion toolkit...',
  'Your files will always stay on your machine.',
  'Setting up Python environment...',
  'Almost there...',
  'Verifying codecs and converters...',
  'No cloud. No uploads. Just fast conversions.',
  'Finalizing setup...',
];

const stage = ref<Stage>('installing');
const checking = ref(false);
const retrying = ref(false);
const currentIndex = ref(0);
const depStates = reactive<DepState[]>([]);
const loadingMessageIndex = ref(0);

let messageTimer: ReturnType<typeof setInterval> | null = null;

const loadingMessage = computed(() => LOADING_MESSAGES[loadingMessageIndex.value % LOADING_MESSAGES.length]);
const totalDeps = computed(() => depStates.length || 1);
const progress = computed(() => {
  if (depStates.length === 0) return 0.05;
  const done = depStates.filter(d => d.status === 'done' || d.status === 'failed').length;
  return done / depStates.length;
});

const os = computed(() => {
  const ua = navigator.userAgent.toLowerCase();
  if (ua.includes('mac')) return 'macos';
  if (ua.includes('win')) return 'windows';
  return 'linux';
});

const installCommand = computed(() => {
  const failed = depStates.filter(d => d.status === 'failed').map(d => d.name);
  const brewPkgs = failed.filter(n => !n.startsWith('pip:'));
  const pipModules = failed.filter(n => n.startsWith('pip:')).map(n => n.slice(4));

  if (os.value === 'macos') {
    const parts = [];
    if (brewPkgs.length) parts.push(`brew install ${brewPkgs.join(' ')}`);
    if (pipModules.length) parts.push(`~/.convx/venv/bin/pip install ${pipModules.join(' ')}`);
    return parts.join(' && ') || 'Try reinstalling convx from the .pkg installer';
  }
  if (os.value === 'linux') {
    const parts = [];
    if (brewPkgs.length) parts.push(`sudo apt-get install -y ${brewPkgs.join(' ')}`);
    if (pipModules.length) parts.push(`~/.convx/venv/bin/pip install ${pipModules.join(' ')}`);
    return parts.join(' && ') || 'See https://convx.dev/docs for your platform';
  }
  return 'See https://convx.dev/docs for your platform';
});

function startMessageCycler() {
  stopMessageCycler();
  loadingMessageIndex.value = 0;
  messageTimer = setInterval(() => {
    loadingMessageIndex.value++;
  }, 4000);
}

function stopMessageCycler() {
  if (messageTimer) {
    clearInterval(messageTimer);
    messageTimer = null;
  }
}

onUnmounted(stopMessageCycler);

async function autoInstall() {
  stage.value = 'installing';
  depStates.length = 0;
  currentIndex.value = 0;
  startMessageCycler();

  try {
    const bridge = await getBridge();

    // Quick check — maybe everything is there already
    const check = await bridge.checkDependencies();
    if (check.ok) {
      stopMessageCycler();
      stage.value = 'ready';
      emit('ready');
      return;
    }

    // Get the missing list
    const missing = await bridge.getMissingDependencies();
    if (missing.length === 0) {
      stopMessageCycler();
      stage.value = 'ready';
      emit('ready');
      return;
    }

    // Populate states
    for (const name of missing) {
      depStates.push({
        name,
        label: FRIENDLY_NAMES[name] ?? name,
        status: 'pending',
      });
    }

    // Install one-by-one
    for (let i = 0; i < depStates.length; i++) {
      currentIndex.value = i;
      depStates[i].status = 'installing';

      const result = await bridge.installSingleDependency(depStates[i].name);

      if (result.ok) {
        depStates[i].status = 'done';
      } else {
        depStates[i].status = 'failed';
        depStates[i].error = result.message;
      }
    }

    stopMessageCycler();

    // Final verify
    const finalCheck = await bridge.checkDependencies();
    if (finalCheck.ok) {
      stage.value = 'ready';
      emit('ready');
    } else {
      stage.value = 'failed';
    }
  } catch {
    stopMessageCycler();
    stage.value = 'failed';
  }
}

async function retryInstall() {
  retrying.value = true;
  await autoInstall();
  retrying.value = false;
}

async function recheck() {
  checking.value = true;
  try {
    const bridge = await getBridge();
    const status = await bridge.checkDependencies();
    if (status.ok) {
      stage.value = 'ready';
      emit('ready');
    } else {
      Notify.create({ type: 'warning', message: 'Some dependencies are still missing' });
    }
  } catch {
    // ignore
  } finally {
    checking.value = false;
  }
}

async function copyCommand() {
  try {
    await navigator.clipboard.writeText(installCommand.value);
    Notify.create({ type: 'positive', message: 'Copied to clipboard' });
  } catch {
    Notify.create({ type: 'negative', message: 'Could not copy' });
  }
}

async function openDocs() {
  try {
    await open('https://convx.dev/docs');
  } catch {
    window.open('https://convx.dev/docs', '_blank');
  }
}

function closeWizard() {
  emit('update:modelValue', false);
}

watch(
  () => props.modelValue,
  (open) => {
    if (open) {
      void autoInstall();
    }
  },
  { immediate: true },
);
</script>

<style scoped lang="scss">
.wizard-card {
  min-height: 100vh;
  border-radius: 0;
  padding: 32px;
  background: radial-gradient(circle at 70% 10%, rgba($primary, 0.18), transparent 45%), $dark;
}

.wizard-content {
  max-width: 520px;
  margin: 0 auto;
}

.wizard-center {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  gap: 8px;
  padding-top: 18vh;
}

.wizard-manual {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding-top: 60px;
}

.wizard-title {
  font-size: 28px;
  font-weight: 700;
  color: white;
  margin-bottom: 2px;
}

.wizard-message {
  font-size: 15px;
  min-height: 24px;
  transition: opacity 0.3s ease;
  margin-bottom: 8px;
}

.wizard-progress {
  max-width: 400px;
  width: 100%;
}

.wizard-dep-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-top: 20px;
  width: 100%;
  max-width: 320px;
  text-align: left;
}

.wizard-dep-row {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 14px;
  color: rgba(255, 255, 255, 0.87);
}

.wizard-dep-dot {
  width: 18px;
  height: 18px;
  display: flex;
  align-items: center;
  justify-content: center;

  &::after {
    content: '';
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.2);
  }
}

.wizard-section {
  padding: 16px;

  &__title {
    font-size: 12px;
    letter-spacing: 0.6px;
    text-transform: uppercase;
    color: rgba(255, 255, 255, 0.6);
    margin-bottom: 10px;
  }
}

.wizard-command {
  padding: 10px 12px;
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.04);
  font-size: 12px;
  color: rgba(255, 255, 255, 0.8);
  word-break: break-word;
}

.wizard-actions {
  margin-top: 10px;
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
</style>
