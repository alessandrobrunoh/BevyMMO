# Build stage
FROM node:24.15-alpine AS builder

WORKDIR /app

ENV NG_CLI_ANALYTICS=false

# Install dependencies first for better layer caching
COPY package.json package-lock.json ./
RUN npm ci

# Copy application source code and build for production
COPY . .
RUN npm run build

# Runtime stage: serve via Nginx
FROM nginx:alpine AS runtime

COPY nginx.conf /etc/nginx/conf.d/default.conf
COPY --from=builder /app/dist/eivar-frontend/browser /usr/share/nginx/html

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]
