/**
 * Browser-facing gateway prefix used by every versioned API route.
 *
 * Docker serves Angular and proxies `/v1` through Nginx to `apps/gateway`, so
 * the browser never bakes in a machine-specific host like `localhost` or a
 * production IP. This keeps local Docker, remote Docker, and reverse-proxy
 * deployments on the same artifact.
 *
 * @example
 * // AuthService builds `/v1/profile`, and Nginx forwards it to the gateway.
 * const profileUrl = `${API_BASE_URL}/profile`;
 */
export const API_BASE_URL = '/v1';
