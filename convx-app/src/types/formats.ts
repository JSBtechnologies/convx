export type FormatCategory = 'image' | 'video' | 'audio' | 'document' | 'data' | 'ebook';

export interface FormatInfo {
  extension: string;
  label: string;
  category: FormatCategory;
}

export const FORMAT_CATEGORIES: Record<FormatCategory, FormatInfo[]> = {
  image: [
    { extension: 'png', label: 'PNG', category: 'image' },
    { extension: 'jpg', label: 'JPG', category: 'image' },
    { extension: 'webp', label: 'WebP', category: 'image' },
    { extension: 'gif', label: 'GIF', category: 'image' },
    { extension: 'bmp', label: 'BMP', category: 'image' },
    { extension: 'tiff', label: 'TIFF', category: 'image' },
    { extension: 'avif', label: 'AVIF', category: 'image' },
    { extension: 'heic', label: 'HEIC', category: 'image' },
    { extension: 'heif', label: 'HEIF', category: 'image' },
    { extension: 'ico', label: 'ICO', category: 'image' },
    { extension: 'svg', label: 'SVG', category: 'image' },
  ],
  video: [
    { extension: 'mp4', label: 'MP4', category: 'video' },
    { extension: 'webm', label: 'WebM', category: 'video' },
    { extension: 'mov', label: 'MOV', category: 'video' },
    { extension: 'avi', label: 'AVI', category: 'video' },
    { extension: 'mkv', label: 'MKV', category: 'video' },
    { extension: 'wmv', label: 'WMV', category: 'video' },
    { extension: 'flv', label: 'FLV', category: 'video' },
    { extension: 'm4v', label: 'M4V', category: 'video' },
    { extension: 'gif', label: 'GIF', category: 'video' },
  ],
  audio: [
    { extension: 'mp3', label: 'MP3', category: 'audio' },
    { extension: 'wav', label: 'WAV', category: 'audio' },
    { extension: 'flac', label: 'FLAC', category: 'audio' },
    { extension: 'm4a', label: 'M4A', category: 'audio' },
    { extension: 'aac', label: 'AAC', category: 'audio' },
    { extension: 'ogg', label: 'OGG', category: 'audio' },
    { extension: 'wma', label: 'WMA', category: 'audio' },
    { extension: 'aiff', label: 'AIFF', category: 'audio' },
    { extension: 'opus', label: 'Opus', category: 'audio' },
  ],
  document: [
    { extension: 'pdf', label: 'PDF', category: 'document' },
    { extension: 'docx', label: 'DOCX', category: 'document' },
    { extension: 'pptx', label: 'PPTX', category: 'document' },
    { extension: 'xlsx', label: 'XLSX', category: 'document' },
    { extension: 'txt', label: 'TXT', category: 'document' },
    { extension: 'md', label: 'Markdown', category: 'document' },
    { extension: 'html', label: 'HTML', category: 'document' },
  ],
  data: [
    { extension: 'csv', label: 'CSV', category: 'data' },
    { extension: 'json', label: 'JSON', category: 'data' },
    { extension: 'yaml', label: 'YAML', category: 'data' },
    { extension: 'xml', label: 'XML', category: 'data' },
    { extension: 'parquet', label: 'Parquet', category: 'data' },
    { extension: 'jsonl', label: 'JSONL', category: 'data' },
    { extension: 'tsv', label: 'TSV', category: 'data' },
    { extension: 'arrow', label: 'Arrow', category: 'data' },
    { extension: 'sqlite', label: 'SQLite', category: 'data' },
    { extension: 'npy', label: 'NPY', category: 'data' },
    { extension: 'npz', label: 'NPZ', category: 'data' },
    { extension: 'h5', label: 'HDF5', category: 'data' },
  ],
  ebook: [
    { extension: 'epub', label: 'EPUB', category: 'ebook' },
    { extension: 'mobi', label: 'MOBI', category: 'ebook' },
  ],
};

export function getFormatCategory(ext: string): FormatCategory | null {
  for (const [category, formats] of Object.entries(FORMAT_CATEGORIES)) {
    if (formats.some((f) => f.extension === ext.toLowerCase())) {
      return category as FormatCategory;
    }
  }
  return null;
}
