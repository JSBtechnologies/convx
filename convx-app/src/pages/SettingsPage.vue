<template>
  <q-page class="settings-page" padding>
    <div class="settings-page__container">
      <h1 class="settings-page__title">Settings</h1>

      <div class="settings-section glass-card">
        <div class="settings-section__title">Defaults</div>

        <div class="settings-row">
          <div class="settings-row__label">Default Quality</div>
          <div class="settings-row__control" style="width: 200px">
            <q-slider
              v-model="settings.defaultQuality"
              :min="10"
              :max="100"
              :step="5"
              label
              color="primary"
              aria-label="Default quality"
            />
          </div>
          <span class="mono" style="font-size: 13px; width: 40px; text-align: right">
            {{ settings.defaultQuality }}%
          </span>
        </div>

        <div class="settings-row">
          <div class="settings-row__label">Default Format</div>
          <q-select
            v-model="settings.defaultFormat"
            :options="formatOptions"
            dense
            outlined
            dark
            style="width: 140px"
            aria-label="Default format"
          />
        </div>

        <div class="settings-row">
          <div class="settings-row__label">Overwrite Files by Default</div>
          <q-toggle
            v-model="settings.overwriteExisting"
            color="primary"
            aria-label="Overwrite files by default"
          />
        </div>

        <div class="settings-row">
          <div class="settings-row__label">Show Notifications</div>
          <q-toggle
            v-model="settings.showNotifications"
            color="primary"
            aria-label="Show notifications"
          />
        </div>
      </div>

      <div class="settings-section glass-card">
        <div class="settings-section__title">System</div>

        <div class="settings-row">
          <div class="settings-row__label">Dependencies</div>
          <q-btn
            flat
            no-caps
            size="sm"
            color="primary"
            label="Check"
            :loading="checking"
            @click="checkDeps"
          />
        </div>

        <div v-if="depsResult" class="settings-deps mono" style="font-size: 12px">
          <q-icon
            :name="depsResult.ok ? 'sym_r_check_circle' : 'sym_r_warning'"
            :color="depsResult.ok ? 'positive' : 'warning'"
            size="16px"
          />
          <pre style="margin: 0; white-space: pre-wrap">{{ depsResult.message }}</pre>
        </div>

        <div class="settings-row">
          <div class="settings-row__label">Version</div>
          <span class="mono text-glass" style="font-size: 13px">1.0.0</span>
        </div>
      </div>

      <div class="settings-section glass-card">
        <div class="settings-section__title">Help</div>

        <div class="settings-row">
          <div class="settings-row__label">Project</div>
          <q-btn
            flat
            no-caps
            size="sm"
            color="primary"
            label="GitHub"
            icon="sym_r_open_in_new"
            @click="openUrl('https://github.com/JSBtechnologies/convx')"
          />
        </div>

        <div class="settings-row">
          <div class="settings-row__label">Issues &amp; Support</div>
          <q-btn
            flat
            no-caps
            size="sm"
            color="primary"
            label="Open Issues"
            icon="sym_r_open_in_new"
            @click="openUrl('https://github.com/JSBtechnologies/convx/issues')"
          />
        </div>
      </div>

      <div class="settings-section glass-card">
        <div class="settings-section__title">MCP Server</div>

        <div class="settings-row">
          <div class="settings-row__label">Binary Path</div>
          <span class="mono text-glass" style="font-size: 12px; word-break: break-all">{{ mcpConfig?.binaryPath || '...' }}</span>
        </div>

        <div class="settings-row">
          <div class="settings-row__label">Target</div>
          <q-btn-toggle
            v-model="mcpTarget"
            no-caps
            dense
            rounded
            toggle-color="primary"
            :options="[
              { label: 'Claude Desktop', value: 'claude-desktop' },
              { label: 'Cursor', value: 'cursor' },
            ]"
          />
        </div>

        <div class="settings-mcp-snippet mono">
          <pre style="margin: 0; white-space: pre-wrap; font-size: 12px">{{ mcpTarget === 'cursor' ? mcpConfig?.cursor : mcpConfig?.claudeDesktop }}</pre>
        </div>

        <div class="settings-row" style="gap: 8px; justify-content: flex-end">
          <q-btn
            flat
            no-caps
            size="sm"
            color="primary"
            icon="sym_r_content_copy"
            label="Copy"
            @click="copyMcpConfig"
          />
          <q-btn
            flat
            no-caps
            size="sm"
            color="primary"
            icon="sym_r_settings"
            label="Auto-Configure"
            :loading="mcpConfiguring"
            @click="autoConfigureMcp"
          />
        </div>
      </div>
    </div>
  </q-page>
</template>

<script setup lang="ts">
import { Notify } from 'quasar';
import { onMounted, ref } from 'vue';
import { getBridge } from '../services/bridge';
import { useSettingsStore } from '../stores/settings';

const settings = useSettingsStore();
const checking = ref(false);
const depsResult = ref<{ ok: boolean; message: string } | null>(null);
const mcpConfig = ref<{ binaryPath: string; claudeDesktop: string; cursor: string } | null>(null);
const mcpTarget = ref<'claude-desktop' | 'cursor'>('claude-desktop');
const mcpConfiguring = ref(false);

const formatOptions = [
  'webp', 'png', 'jpg', 'gif', 'avif',
  'mp4', 'webm', 'mov',
  'mp3', 'wav', 'flac', 'm4a', 'aac', 'ogg',
];

async function openUrl(url: string) {
  try {
    const { open } = await import('@tauri-apps/plugin-shell');
    await open(url);
  } catch {
    window.open(url, '_blank');
  }
}

onMounted(async () => {
  const bridge = await getBridge();
  try {
    mcpConfig.value = await bridge.getMcpConfig();
  } catch {
    // ignore — MCP config not critical
  }
});

async function checkDeps() {
  checking.value = true;
  try {
    const bridge = await getBridge();
    depsResult.value = await bridge.checkDependencies();
  } catch (e) {
    depsResult.value = { ok: false, message: String(e) };
  } finally {
    checking.value = false;
  }
}

async function copyMcpConfig() {
  const text = mcpTarget.value === 'cursor' ? mcpConfig.value?.cursor : mcpConfig.value?.claudeDesktop;
  if (!text) return;
  try {
    await navigator.clipboard.writeText(text);
    Notify.create({ type: 'positive', message: 'Config copied to clipboard' });
  } catch {
    Notify.create({ type: 'negative', message: 'Failed to copy' });
  }
}

async function autoConfigureMcp() {
  mcpConfiguring.value = true;
  try {
    const bridge = await getBridge();
    await bridge.autoConfigureMcp(mcpTarget.value);
    Notify.create({ type: 'positive', message: `MCP server configured. Restart ${mcpTarget.value === 'claude-desktop' ? 'Claude Desktop' : 'Cursor'} to apply.`, timeout: 5000 });
  } catch (e) {
    Notify.create({ type: 'negative', message: `Auto-configure failed: ${String(e)}` });
  } finally {
    mcpConfiguring.value = false;
  }
}
</script>

<style lang="scss" scoped>
.settings-page {
  position: relative;
  z-index: 1;
  padding: 24px 32px;

  &__container {
    max-width: 700px;
    margin: 0 auto;
  }

  &__title {
    margin: 0 0 24px;
    font-size: 22px;
    font-weight: 600;
    color: rgba(255, 255, 255, 0.85);
  }
}

.settings-section {
  padding: 24px;
  margin-bottom: 20px;
  display: flex;
  flex-direction: column;
  gap: 16px;

  &__title {
    font-size: 13px;
    font-weight: 600;
    color: rgba(255, 255, 255, 0.5);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }
}

.settings-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;

  &__label {
    font-size: 14px;
    color: rgba(255, 255, 255, 0.7);
  }
}

.settings-deps {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 12px;
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.03);
  color: rgba(255, 255, 255, 0.6);
}

.settings-mcp-snippet {
  padding: 12px;
  border-radius: 8px;
  background: rgba(0, 0, 0, 0.3);
  color: rgba(255, 255, 255, 0.7);
  border: 1px solid rgba(255, 255, 255, 0.06);
  max-height: 200px;
  overflow-y: auto;
}
</style>
