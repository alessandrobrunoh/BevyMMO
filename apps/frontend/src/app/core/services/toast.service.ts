import { Injectable, signal } from '@angular/core';

export interface ToastMessage {
  id: string;
  type: 'info' | 'success' | 'warning' | 'rune';
  title?: string;
  message: string;
  duration?: number;
}

@Injectable({
  providedIn: 'root'
})
export class ToastService {
  readonly toasts = signal<ToastMessage[]>([]);

  show(toast: Omit<ToastMessage, 'id'>) {
    const id = Math.random().toString(36).substring(2, 9);
    const newToast: ToastMessage = {
      ...toast,
      id,
      duration: toast.duration || 4000
    };

    this.toasts.update(current => [...current, newToast]);

    setTimeout(() => {
      this.dismiss(id);
    }, newToast.duration);
  }

  showInfo(message: string, title?: string) {
    this.show({ type: 'info', title, message });
  }

  showSuccess(message: string, title?: string) {
    this.show({ type: 'success', title, message });
  }

  showWarning(message: string, title?: string) {
    this.show({ type: 'warning', title, message });
  }

  showRunic(message: string, title?: string) {
    this.show({ type: 'rune', title, message });
  }

  dismiss(id: string) {
    this.toasts.update(current => current.filter(t => t.id !== id));
  }
}
