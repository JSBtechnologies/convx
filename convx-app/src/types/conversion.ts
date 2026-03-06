export interface ConversionOptions {
  outputFormat: string;
  quality?: number;
  width?: number;
  height?: number;
  fps?: number;
  crf?: number;
  noAudio?: boolean;
  bitrate?: string;
  sampleRate?: number;
  overwrite?: boolean;
}

export interface ConversionResult {
  id: string;
  status: 'completed' | 'failed';
  inputPath: string;
  outputPath?: string;
  inputFormat: string;
  outputFormat: string;
  inputSize: number;
  outputSize?: number;
  spaceSaved?: number;
  durationMs: number;
  error?: string;
  timestamp: string;
}

export interface FileInfo {
  path: string;
  name: string;
  extension: string;
  size: number;
}

export type ConversionStage =
  | 'idle'
  | 'reading'
  | 'converting'
  | 'writing'
  | 'complete'
  | 'error';

export interface ConversionProgress {
  stage: ConversionStage;
  percent: number;
  message?: string;
}
