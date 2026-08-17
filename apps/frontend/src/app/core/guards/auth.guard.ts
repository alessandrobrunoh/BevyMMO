import { inject } from '@angular/core';
import { CanActivateFn, Router } from '@angular/router';
import { AuthService } from '../services/auth.service';

/**
 * Redirects anonymous visitors to `/login`. Safe to check synchronously:
 * `AuthService.restoreSession()` runs as an app initializer
 * (`app.config.ts`), which Angular's bootstrap waits on before routing
 * starts — by the time any guard runs, a valid session cookie has already
 * been resolved into `isLoggedIn()`.
 */
export const authGuard: CanActivateFn = () => {
  const authService = inject(AuthService);
  const router = inject(Router);

  if (authService.isLoggedIn()) {
    return true;
  }
  return router.createUrlTree(['/login']);
};
