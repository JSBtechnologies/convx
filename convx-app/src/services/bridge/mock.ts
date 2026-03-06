import type {
    ConversionOptions,
    ConversionProgress,
    ConversionResult,
} from 'src/types/conversion';
import type { ActivateResult, EnterpriseConfig, LicenseInfo, LicenseStatus } from 'src/types/license';

export class MockBridge {
  private progressCallbacks: Array<(p: ConversionProgress) => void> = [];

  async init(): Promise<void> {
    console.log('[MockBridge] Initialized -- no Tauri backend');
  }

  async convert(
    inputPath: string,
    outputPath: string,
    options: ConversionOptions,
  ): Promise<ConversionResult> {
    const stages: Array<{
      stage: ConversionProgress['stage'];
      percent: number;
    }> = [
      { stage: 'reading', percent: 10 },
      { stage: 'converting', percent: 50 },
      { stage: 'converting', percent: 75 },
      { stage: 'writing', percent: 90 },
      { stage: 'complete', percent: 100 },
    ];

    for (const s of stages) {
      await new Promise((r) => setTimeout(r, 400));
      for (const cb of this.progressCallbacks) {
        cb({ stage: s.stage, percent: s.percent });
      }
    }

    const inputSize = 2_400_000;
    const outputSize = 340_000;

    return {
      id: crypto.randomUUID(),
      status: 'completed',
      inputPath,
      outputPath,
      inputFormat: inputPath.split('.').pop() || 'unknown',
      outputFormat: options.outputFormat,
      inputSize,
      outputSize,
      spaceSaved: inputSize - outputSize,
      durationMs: 2000,
      timestamp: new Date().toISOString(),
    };
  }

  async cancelConversion(): Promise<boolean> {
    return true;
  }

  async canConvert(_from: string, _to: string): Promise<boolean> {
    return true;
  }

  async getSupportedFormats(): Promise<string[]> {
    return [
      'png', 'jpg', 'webp', 'gif', 'bmp', 'tiff', 'ico', 'svg', 'avif', 'heic', 'heif',
      'mp4', 'webm', 'mov', 'avi', 'mkv', 'wmv', 'flv', 'm4v',
      'mp3', 'wav', 'flac', 'm4a', 'aac', 'ogg', 'wma', 'aiff', 'opus',
      'csv', 'json', 'yaml', 'xml', 'parquet', 'jsonl', 'tsv', 'arrow', 'sqlite', 'npy', 'npz', 'h5',
    ];
  }

  async getConversionTargets(from: string): Promise<string[]> {
    const imageFormats = ['png', 'jpg', 'webp', 'gif', 'bmp', 'tiff', 'ico', 'svg', 'avif', 'heic', 'heif'];
    const videoFormats = ['mp4', 'webm', 'mov', 'avi', 'mkv', 'wmv', 'flv', 'm4v', 'gif'];
    const audioFormats = ['mp3', 'wav', 'flac', 'm4a', 'aac', 'ogg', 'wma', 'aiff', 'opus'];
    const dataTargets: Record<string, string[]> = {
      csv: ['json', 'yaml', 'tsv', 'jsonl', 'xlsx', 'parquet', 'arrow', 'html', 'pdf', 'md'],
      json: ['csv', 'yaml', 'xml', 'jsonl', 'parquet', 'arrow', 'html', 'pdf', 'md'],
      yaml: ['json', 'html', 'pdf'],
      xml: ['json', 'html', 'pdf'],
      tsv: ['csv', 'html', 'pdf'],
      jsonl: ['json', 'csv', 'html', 'pdf'],
      parquet: ['csv', 'json'],
      arrow: ['csv', 'json'],
      sqlite: ['csv', 'json'],
      npy: ['csv'],
      npz: ['csv'],
      h5: ['csv', 'json'],
    };

    if (imageFormats.includes(from)) return imageFormats.filter((f) => f !== from);
    if (videoFormats.includes(from)) return videoFormats.filter((f) => f !== from);
    if (audioFormats.includes(from)) return audioFormats.filter((f) => f !== from);
    if (from in dataTargets) return dataTargets[from];
    return [];
  }

  async checkDependencies(): Promise<{ ok: boolean; message: string }> {
    return { ok: true, message: 'Mock mode -- dependencies not checked' };
  }

  async installDependencies(): Promise<{ ok: boolean; message: string }> {
    return { ok: true, message: 'Mock mode -- dependencies treated as installed' };
  }

  async getMissingDependencies(): Promise<string[]> {
    return [];
  }

  async installSingleDependency(_name: string): Promise<{ ok: boolean; message: string }> {
    return { ok: true, message: 'Mock mode -- installed' };
  }

  async ensurePostInstall(): Promise<{ ok: boolean; repairs: string[] }> {
    return { ok: true, repairs: [] };
  }

  async getFileInfo(
    path: string,
  ): Promise<{ name: string; size: number; extension: string }> {
    const name = path.split('/').pop() || path;
    return {
      name,
      size: 2_400_000,
      extension: name.split('.').pop() || '',
    };
  }

  async pathExists(_path: string): Promise<boolean> {
    return false;
  }

  async revealInFolder(_path: string): Promise<void> {
    // noop in browser/mock mode
  }

  // ─── License (mock always returns valid) ───────────────────────────

  async checkLicense(): Promise<LicenseStatus> {
    return { status: 'valid', device_name: 'Mock Device', recheck_after: new Date(Date.now() + 30 * 86_400_000).toISOString() };
  }

  async activateLicense(_key: string): Promise<ActivateResult> {
    return { outcome: 'activated', device_name: 'Mock Device' };
  }

  async transferLicense(_key: string): Promise<boolean> {
    return true;
  }

  async deactivateLicense(): Promise<boolean> {
    return true;
  }

  async getLicenseInfo(): Promise<LicenseInfo | null> {
    return {
      key_masked: 'CONVX-****-****-****-MOCK',
      device_name: 'Mock Device',
      platform: 'macos',
      activated_at: new Date().toISOString(),
      recheck_after: new Date(Date.now() + 30 * 86_400_000).toISOString(),
    };
  }

  // ─── Enterprise (mock returns no enterprise config) ─────────────────

  async getEnterpriseConfig(): Promise<EnterpriseConfig> {
    return { has_config: false };
  }

  async autoActivate(): Promise<ActivateResult> {
    return { outcome: 'no_config' };
  }

  async sendConversionAudit(_data: {
    inputFormat: string;
    outputFormat: string;
    inputSize: number;
    outputSize: number;
    durationMs: number;
  }): Promise<void> {
    // noop in mock mode
  }

  // ─── MCP Server ────────────────────────────────────────────────────

  async getMcpConfig(): Promise<{ binaryPath: string; claudeDesktop: string; cursor: string }> {
    return {
      binaryPath: '/Applications/ConvX.app/Contents/MacOS/ConvX',
      claudeDesktop: JSON.stringify({ mcpServers: { convx: { command: '/Applications/ConvX.app/Contents/MacOS/ConvX', args: ['--mcp'] } } }, null, 2),
      cursor: JSON.stringify({ mcpServers: { convx: { command: '/Applications/ConvX.app/Contents/MacOS/ConvX', args: ['--mcp'] } } }, null, 2),
    };
  }

  async autoConfigureMcp(_target: string): Promise<string> {
    return '/mock/path/config.json';
  }

  onProgress(callback: (p: ConversionProgress) => void): void {
    this.progressCallbacks.push(callback);
  }

  offProgress(callback: (p: ConversionProgress) => void): void {
    this.progressCallbacks = this.progressCallbacks.filter(
      (cb) => cb !== callback,
    );
  }

  destroy(): void {
    // noop
  }
}
