import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type {
    ConversionOptions,
    ConversionProgress,
    ConversionResult,
} from 'src/types/conversion';

export class TauriBridge {
  private progressCallbacks: Array<(p: ConversionProgress) => void> = [];
  private unlistenProgress: (() => void) | null = null;

  async init(): Promise<void> {
    const unlisten = await listen<ConversionProgress>(
      'conversion-progress',
      (event) => {
        for (const cb of this.progressCallbacks) {
          cb(event.payload);
        }
      },
    );
    this.unlistenProgress = unlisten;
  }

  async convert(
    inputPath: string,
    outputPath: string,
    options: ConversionOptions,
  ): Promise<ConversionResult> {
    return invoke<ConversionResult>('convert_file', {
      input: inputPath,
      output: outputPath,
      options: {
        output_format: options.outputFormat,
        quality: options.quality ?? null,
        width: options.width ?? null,
        height: options.height ?? null,
        overwrite: options.overwrite ?? false,
      },
    });
  }

  async cancelConversion(): Promise<boolean> {
    return invoke<boolean>('cancel_conversion');
  }

  async canConvert(from: string, to: string): Promise<boolean> {
    return invoke<boolean>('can_convert', { from, to });
  }

  async getSupportedFormats(): Promise<string[]> {
    return invoke<string[]>('get_supported_formats');
  }

  async getConversionTargets(from: string): Promise<string[]> {
    return invoke<string[]>('get_conversion_targets', { from });
  }

  async checkDependencies(): Promise<{ ok: boolean; message: string }> {
    return invoke<{ ok: boolean; message: string }>('check_dependencies');
  }

  async installDependencies(): Promise<{ ok: boolean; message: string }> {
    return invoke<{ ok: boolean; message: string }>('install_dependencies');
  }

  async getMissingDependencies(): Promise<string[]> {
    return invoke<string[]>('get_missing_dependencies');
  }

  async installSingleDependency(name: string): Promise<{ ok: boolean; message: string }> {
    return invoke<{ ok: boolean; message: string }>('install_single_dependency', { name });
  }

  async ensurePostInstall(): Promise<{ ok: boolean; repairs: string[] }> {
    return invoke<{ ok: boolean; repairs: string[] }>('ensure_post_install');
  }

  async getFileInfo(
    path: string,
  ): Promise<{ name: string; size: number; extension: string }> {
    return invoke<{ name: string; size: number; extension: string }>(
      'get_file_info',
      { path },
    );
  }

  async pathExists(path: string): Promise<boolean> {
    return invoke<boolean>('path_exists', { path });
  }

  async revealInFolder(path: string): Promise<void> {
    await invoke('reveal_in_file_manager', { path });
  }

  // ─── MCP Server ────────────────────────────────────────────────────

  async getMcpConfig(): Promise<{ binaryPath: string; claudeDesktop: string; cursor: string }> {
    const result = await invoke<{ binary_path: string; claude_desktop: string; cursor: string }>('get_mcp_config');
    return {
      binaryPath: result.binary_path,
      claudeDesktop: result.claude_desktop,
      cursor: result.cursor,
    };
  }

  async autoConfigureMcp(target: string): Promise<string> {
    return invoke<string>('auto_configure_mcp', { target });
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
    if (this.unlistenProgress) {
      this.unlistenProgress();
    }
  }
}
