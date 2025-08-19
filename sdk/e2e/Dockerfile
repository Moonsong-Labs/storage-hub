FROM mcr.microsoft.com/playwright:v1.54.2-jammy

RUN corepack enable && corepack prepare pnpm@10 --activate

WORKDIR /work/sdk/e2e

# Install only E2E dependencies first for better layer caching
COPY package.json pnpm-lock.yaml ./
RUN pnpm install

# Copy E2E sources (page, tests, config)
COPY . ./

# Ensure Chromium and system deps are installed for Playwright
RUN pnpm exec playwright install --with-deps chromium

# Run headed inside Xvfb to support extensions (MetaMask)
ENV HEADLESS=false

EXPOSE 3000

CMD bash -lc '\
  set -euo pipefail; \
  nohup python3 -m http.server 3000 --directory page >/dev/null 2>&1 & \
  echo "Waiting for http://localhost:3000 ..."; \
  for i in {1..60}; do curl -sSf http://localhost:3000 >/dev/null && break || sleep 1; done; \
  xvfb-run -a pnpm exec playwright test \
'


