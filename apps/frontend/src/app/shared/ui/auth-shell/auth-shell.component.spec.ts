import { ComponentFixture, TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { AuthShellComponent } from './auth-shell.component';

describe('AuthShellComponent', () => {
  let fixture: ComponentFixture<AuthShellComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [AuthShellComponent],
      providers: [provideRouter([])]
    }).compileComponents();

    fixture = TestBed.createComponent(AuthShellComponent);
  });

  it('renders the supplied auth heading', () => {
    fixture.componentRef.setInput('mode', 'login');
    fixture.componentRef.setInput('title', 'Enter Eivar');
    fixture.detectChanges();

    expect(fixture.nativeElement.querySelector('h1').textContent).toContain('Enter Eivar');
  });
});
