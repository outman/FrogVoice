import { ref } from 'vue';

export function useDragDrop() {
  const isDragging = ref(false);
  const path = ref('');
  const error = ref('');

  function onDragOver(e: DragEvent) {
    e.preventDefault();
    e.stopPropagation();
    isDragging.value = true;
  }

  function onDragLeave(e: DragEvent) {
    e.preventDefault();
    e.stopPropagation();
    isDragging.value = false;
  }

  function onDrop(e: DragEvent): string | null {
    e.preventDefault();
    e.stopPropagation();
    isDragging.value = false;
    error.value = '';

    if (!e.dataTransfer?.files.length) return null;

    // Tauri's WebView adds a `path` property to File objects
    const file = e.dataTransfer.files[0] as File & { path?: string };
    const filePath = file.path || '';

    if (!filePath) {
      error.value = '无法获取文件夹路径';
      return null;
    }

    path.value = filePath;
    return filePath;
  }

  return {
    isDragging,
    path,
    error,
    onDragOver,
    onDragLeave,
    onDrop,
  };
}
