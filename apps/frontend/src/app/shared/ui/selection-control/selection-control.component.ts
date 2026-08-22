import { Component, EventEmitter, forwardRef, Input, Output } from '@angular/core';
import { ControlValueAccessor, NG_VALUE_ACCESSOR } from '@angular/forms';

export type SelectionControlKind = 'checkbox' | 'radio' | 'switch';

@Component({
  selector: 'app-eivar-selection-control',
  standalone: true,
  providers: [
    {
      provide: NG_VALUE_ACCESSOR,
      useExisting: forwardRef(() => EivarSelectionControlComponent),
      multi: true
    }
  ],
  template: `
    <label class="eivar-selection" [class.is-disabled]="disabled" [class.is-checked]="checked">
      <input
        [type]="kind === 'switch' ? 'checkbox' : kind"
        [name]="name || null"
        [checked]="checked"
        [disabled]="disabled"
        [attr.aria-label]="ariaLabel || label || null"
        (change)="onNativeChange($event)"
        (blur)="onTouched()"
      />
      <span class="eivar-selection__control" [class]="'eivar-selection__control kind-' + kind" aria-hidden="true">
        @if (kind === 'checkbox') { <span class="eivar-selection__mark">✓</span> }
        @if (kind === 'switch') { <span class="eivar-selection__thumb">✦</span> }
      </span>
      @if (label) { <span class="eivar-selection__label">{{ label }}</span> }
    </label>
  `,
  styleUrls: ['./selection-control.component.scss']
})
export class EivarSelectionControlComponent implements ControlValueAccessor {
  @Input() kind: SelectionControlKind = 'checkbox';
  @Input() label?: string;
  @Input() name?: string;
  @Input() ariaLabel?: string;
  @Input() disabled = false;
  @Input() checked = false;

  @Output() checkedChange = new EventEmitter<boolean>();

  private onChange: (checked: boolean) => void = () => undefined;
  onTouched: () => void = () => undefined;

  writeValue(checked: boolean | null): void {
    this.checked = Boolean(checked);
  }

  registerOnChange(fn: (checked: boolean) => void): void {
    this.onChange = fn;
  }

  registerOnTouched(fn: () => void): void {
    this.onTouched = fn;
  }

  setDisabledState(disabled: boolean): void {
    this.disabled = disabled;
  }

  onNativeChange(event: Event): void {
    const checked = (event.target as HTMLInputElement).checked;
    this.checked = checked;
    this.checkedChange.emit(checked);
    this.onChange(checked);
  }
}
