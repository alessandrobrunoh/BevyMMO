/**
 * Base URL of `apps/gateway`, including the API version prefix (`/v1` —
 * every business route of the gateway is versioned; `/`, `/health` and
 * `/docs` are not, but nothing here calls them). Not an Angular "environment"
 * (the project has none yet — see `angular.json`, no `fileReplacements`): a
 * single constant is enough for the one thing that varies today. Override at
 * build time by editing this file per deploy, or by wiring proper environment
 * files if a second target (e.g. a staging gateway) shows up.
 */
export const API_BASE_URL = 'http://localhost:8081/v1';
