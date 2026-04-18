<template>
  <div class="format-selector">
    <div class="format-selector__tabs" role="tablist" aria-label="Format categories">
      <button
        v-for="cat in categories"
        :key="cat.value"
        role="tab"
        :aria-selected="selectedCategory === cat.value"
        :aria-label="cat.label + ' formats'"
        class="format-selector__tab"
        :class="{ 'format-selector__tab--active': selectedCategory === cat.value }"
        @click="selectedCategory = cat.value"
      >
        <q-icon :name="cat.icon" size="16px" aria-hidden="true" />
        {{ cat.label }}
      </button>
    </div>

    <div class="format-selector__grid" role="listbox" :aria-label="selectedCategory + ' format options'">
      <button
        v-for="fmt in filteredFormats"
        :key="fmt.extension"
        role="option"
        :aria-selected="modelValue === fmt.extension"
        :aria-label="'Convert to ' + fmt.label"
        class="format-selector__pill"
        :class="{ 'format-selector__pill--active': modelValue === fmt.extension }"
        @click="emit('update:modelValue', fmt.extension)"
      >
        {{ fmt.label }}
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue';
import { FORMAT_CATEGORIES, getFormatCategory } from 'src/types/formats';
import type { FormatCategory, FormatInfo } from 'src/types/formats';
import { useConvert } from 'src/composables/useConvert';

const props = defineProps<{
  modelValue: string;
  inputExtension: string;
}>();

const emit = defineEmits<{
  'update:modelValue': [format: string];
}>();

const { getTargets } = useConvert();

const ALL_CATEGORIES = [
  { label: 'Image', value: 'image' as FormatCategory, icon: 'sym_r_image' },
  { label: 'Video', value: 'video' as FormatCategory, icon: 'sym_r_movie' },
  { label: 'Audio', value: 'audio' as FormatCategory, icon: 'sym_r_audio_file' },
  { label: 'Document', value: 'document' as FormatCategory, icon: 'sym_r_description' },
  { label: 'Data', value: 'data' as FormatCategory, icon: 'sym_r_database' },
  { label: 'Ebook', value: 'ebook' as FormatCategory, icon: 'sym_r_book' },
];

const categories = computed(() => {
  if (conversionTargets.value.length === 0) return ALL_CATEGORIES;
  const targetSet = new Set(conversionTargets.value);
  return ALL_CATEGORIES.filter((cat) => {
    const formats = FORMAT_CATEGORIES[cat.value] || [];
    return formats.some((f) => targetSet.has(f.extension));
  });
});

const selectedCategory = ref<FormatCategory>(
  getFormatCategory(props.inputExtension) || 'image',
);
const conversionTargets = ref<string[]>([]);

const filteredFormats = computed<FormatInfo[]>(() => {
  const categoryFormats = FORMAT_CATEGORIES[selectedCategory.value] || [];
  if (conversionTargets.value.length === 0) return categoryFormats;
  return categoryFormats.filter((f) =>
    conversionTargets.value.includes(f.extension),
  );
});

watch(
  () => props.inputExtension,
  async (ext) => {
    if (ext) {
      const cat = getFormatCategory(ext);
      if (cat) selectedCategory.value = cat;
      conversionTargets.value = await getTargets(ext);
    }
  },
  { immediate: true },
);
</script>

<style lang="scss" scoped>
.format-selector {
  display: flex;
  flex-direction: column;
  gap: 14px;

  &__tabs {
    display: flex;
    gap: 4px;
    padding: 3px;
    background: rgba(255, 255, 255, 0.03);
    border-radius: 10px;
    border: 1px solid rgba(255, 255, 255, 0.06);
  }

  &__tab {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 7px 12px;
    border: none;
    border-radius: 8px;
    background: transparent;
    color: rgba(255, 255, 255, 0.65);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s ease;

    &:hover {
      color: rgba(255, 255, 255, 0.7);
      background: rgba(255, 255, 255, 0.04);
    }

    &--active {
      color: white;
      background: rgba($primary, 0.2);
    }
  }

  &__grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(72px, 1fr));
    gap: 6px;
  }

  &__pill {
    padding: 8px 4px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 10px;
    background: rgba(255, 255, 255, 0.03);
    color: rgba(255, 255, 255, 0.7);
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.3px;
    cursor: pointer;
    transition: all 0.2s ease;
    text-align: center;

    &:hover {
      border-color: rgba($primary, 0.3);
      background: rgba($primary, 0.08);
      color: rgba(255, 255, 255, 0.8);
    }

    &--active {
      border-color: $primary;
      background: rgba($primary, 0.15);
      color: white;
      box-shadow: 0 0 16px rgba($primary, 0.2);
    }
  }
}
</style>
