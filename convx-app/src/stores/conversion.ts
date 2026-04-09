import { defineStore } from 'pinia';
import type {
    ConversionProgress,
    ConversionResult,
    ConversionStage,
    FileInfo,
} from 'src/types/conversion';
import { computed, ref } from 'vue';

export const useConversionStore = defineStore('conversion', () => {
  const inputFile = ref<FileInfo | null>(null);
  const outputFormat = ref('webp');
  const quality = ref(80);
  const converting = ref(false);
  const progress = ref<ConversionProgress>({ stage: 'idle', percent: 0 });
  const result = ref<ConversionResult | null>(null);
  const error = ref<string | null>(null);

  const stage = computed<ConversionStage>(() => progress.value.stage);
  const hasFile = computed(() => inputFile.value !== null);
  const canConvert = computed(
    () => hasFile.value && outputFormat.value.length > 0 && !converting.value,
  );

  function setInputFile(file: FileInfo) {
    inputFile.value = file;
    result.value = null;
    error.value = null;
    progress.value = { stage: 'idle', percent: 0 };
  }

  function clearInputFile() {
    inputFile.value = null;
    result.value = null;
    error.value = null;
    progress.value = { stage: 'idle', percent: 0 };
  }

  function setOutputFormat(format: string) {
    outputFormat.value = format;
  }

  function setQuality(q: number) {
    quality.value = q;
  }

  function setConverting(v: boolean) {
    converting.value = v;
  }

  function setProgress(p: ConversionProgress) {
    progress.value = p;
  }

  function setResult(r: ConversionResult | null) {
    result.value = r;
  }

  function setError(msg: string | null) {
    error.value = msg;
  }

  function reset() {
    inputFile.value = null;
    outputFormat.value = 'webp';
    quality.value = 80;
    converting.value = false;
    progress.value = { stage: 'idle', percent: 0 };
    result.value = null;
    error.value = null;
  }

  return {
    inputFile, outputFormat, quality, converting, progress, result, error,
    stage, hasFile, canConvert,
    setInputFile, clearInputFile, setOutputFormat, setQuality,
    setConverting, setProgress, setResult, setError, reset,
  };
});
