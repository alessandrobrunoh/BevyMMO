import { ComponentFixture, TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { EivarButtonComponent } from './button.component';

describe('EivarButtonComponent', () => {
  let fixture: ComponentFixture<EivarButtonComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [EivarButtonComponent],
      providers: [provideRouter([])]
    }).compileComponents();

    fixture = TestBed.createComponent(EivarButtonComponent);
  });

  it('renders a native button and forwards its disabled state', () => {
    fixture.componentRef.setInput('disabled', true);
    fixture.componentRef.setInput('variant', 'danger');
    fixture.detectChanges();

    const button = fixture.nativeElement.querySelector('button');

    expect(button).toBeTruthy();
    expect(button.disabled).toBe(true);
    expect(button.classList.contains('variant-danger')).toBe(true);
  });

  it('renders router navigation as a native link', () => {
    fixture.componentRef.setInput('routerLink', '/news');
    fixture.detectChanges();

    const link = fixture.nativeElement.querySelector('a');

    expect(link).toBeTruthy();
    expect(link.getAttribute('href')).toBe('/news');
    expect(fixture.nativeElement.querySelector('button')).toBeNull();
  });

  it('prevents disabled links from emitting clicks', () => {
    const onClick = vi.fn();
    fixture.componentRef.setInput('href', 'https://example.com');
    fixture.componentRef.setInput('disabled', true);
    fixture.componentInstance.onClick.subscribe(onClick);
    fixture.detectChanges();

    fixture.nativeElement.querySelector('a').click();

    expect(onClick).not.toHaveBeenCalled();
  });

  it('exposes toggle state with aria-pressed', () => {
    fixture.componentRef.setInput('toggle', true);
    fixture.componentRef.setInput('active', true);
    fixture.detectChanges();

    expect(fixture.nativeElement.querySelector('button').getAttribute('aria-pressed')).toBe('true');
  });

  it('marks loading controls busy and prevents interaction', () => {
    fixture.componentRef.setInput('loading', true);
    fixture.detectChanges();

    const button = fixture.nativeElement.querySelector('button');

    expect(button.disabled).toBe(true);
    expect(button.getAttribute('aria-busy')).toBe('true');
  });
});
