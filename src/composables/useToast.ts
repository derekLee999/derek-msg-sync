import { readonly, shallowRef } from 'vue'

export interface ToastItem {
  id: number
  text: string
}

let nextToastId = 1

export function useToast() {
  const toastItems = shallowRef<ToastItem[]>([])

  function dismissToast(id: number) {
    toastItems.value = toastItems.value.filter((item) => item.id !== id)
  }

  function showToast(text: string, duration = 1500) {
    const id = nextToastId++
    toastItems.value = [...toastItems.value, { id, text }]

    window.setTimeout(() => {
      dismissToast(id)
    }, duration)
  }

  return {
    toastItems: readonly(toastItems),
    showToast,
    dismissToast,
  }
}
