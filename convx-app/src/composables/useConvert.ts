import { getBridge } from 'src/services/bridge';
import { useConversionStore } from 'src/stores/conversion';
import { useHistoryStore } from 'src/stores/history';
import { useSettingsStore } from 'src/stores/settings';
import type { ConversionOptions, ConversionProgress } from 'src/types/conversion';

type ConvertOutcome =
  | { ok: true }
  | { ok: false; reason: 'output_exists' | 'error'; message: string };

export function useConvert() {
  const store = useConversionStore();
  const history = useHistoryStore();
  const settings = useSettingsStore();

  function extractErrorMessage(err: unknown): string {
    if (typeof err === 'string') return err;
    if (err instanceof Error) return err.message;

    if (typeof err === 'object' && err !== null) {
      const e = err as { message?: unknown; error?: unknown };
      if (typeof e.message === 'string' && e.message.length > 0) return e.message;
      if (typeof e.error === 'string' && e.error.length > 0) return e.error;
      try {
        return JSON.stringify(err);
      } catch {
        return 'Conversion failed';
      }
    }

    return String(err);
  }

  function buildOutputPath(inputPath: string, outputFormat: string): string {
    const lastSep = Math.max(inputPath.lastIndexOf('/'), inputPath.lastIndexOf('\\'));
    const filename = inputPath.slice(lastSep + 1);
    const dotIndex = filename.indexOf('.');
    const stem = dotIndex === -1 ? filename : filename.slice(0, dotIndex);
    return inputPath.slice(0, lastSep + 1) + stem + '.' + outputFormat;
  }

  function getPlannedOutputPath(): string | null {
    if (!store.inputFile) return null;
    return buildOutputPath(store.inputFile.path, store.outputFormat);
  }

  async function pathExists(path: string): Promise<boolean> {
    const bridge = await getBridge();
    return bridge.pathExists(path);
  }

  async function convert(overwriteOverride?: boolean): Promise<ConvertOutcome> {
    if (!store.inputFile) {
      return { ok: false, reason: 'error', message: 'No input file selected' };
    }
    if (!store.outputFormat) {
      return { ok: false, reason: 'error', message: 'No output format selected' };
    }

    const bridge = await getBridge();
    store.setResult(null);
    store.setError(null);
    store.setConverting(true);
    store.setProgress({ stage: 'reading', percent: 5 });

    const inputPath = store.inputFile.path;
    const outputPath = buildOutputPath(inputPath, store.outputFormat);
    const overwrite = overwriteOverride ?? settings.overwriteExisting;

    const options: ConversionOptions = {
      outputFormat: store.outputFormat,
      quality: store.quality,
      overwrite,
    };

    const onProgress = (p: ConversionProgress) => {
      store.setProgress(p);
    };
    bridge.onProgress(onProgress);

    try {
      const result = await bridge.convert(inputPath, outputPath, options);
      store.setResult(result);
      store.setProgress({ stage: 'complete', percent: 100 });
      history.add(result);
      return { ok: true };
    } catch (err: unknown) {
      const message = extractErrorMessage(err);
      const lower = message.toLowerCase();

      if (lower.includes('cancelled')) {
        store.setError('Conversion cancelled');
        store.setProgress({ stage: 'error', percent: 0, message: 'Conversion cancelled' });
        return { ok: false, reason: 'error', message: 'Conversion cancelled' };
      }

      if (lower.includes('output file already exists')) {
        store.setError(null);
        store.setProgress({ stage: 'idle', percent: 0 });
        return { ok: false, reason: 'output_exists', message };
      }

      store.setError(message);
      store.setProgress({ stage: 'error', percent: 0, message });
      return { ok: false, reason: 'error', message };
    } finally {
      bridge.offProgress(onProgress);
      store.setConverting(false);
    }
  }

  async function getTargets(fromExtension: string): Promise<string[]> {
    const bridge = await getBridge();
    return bridge.getConversionTargets(fromExtension);
  }

  async function cancel(): Promise<boolean> {
    const bridge = await getBridge();
    return bridge.cancelConversion();
  }

  return { convert, cancel, getTargets, getPlannedOutputPath, pathExists };
}
