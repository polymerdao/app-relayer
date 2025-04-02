# Build stage
FROM node:18-alpine as builder

WORKDIR /app

# Copy package files and install dependencies
COPY ts-relayer/package.json ts-relayer/yarn.lock ./
RUN yarn install --frozen-lockfile

# Copy source code
COPY ts-relayer/tsconfig.json ./
COPY ts-relayer/src ./src

# Build the application
RUN yarn build

# Runtime stage
FROM node:18-alpine

WORKDIR /app

# Create a non-root user to run the application
RUN addgroup -S relayer && adduser -S relayer -G relayer

# Copy built app and dependencies from builder stage
COPY --from=builder /app/dist ./dist
COPY --from=builder /app/node_modules ./node_modules
COPY ts-relayer/package.json ./

# Create config directory and ensure it's writable by the non-root user
RUN mkdir -p /app/config && chown -R relayer:relayer /app

# Switch to non-root user
USER relayer

# Set environment variables (defaults, can be overridden at runtime)
ENV NODE_ENV=production
ENV CONFIG_PATH=/app/config/config.yaml

# Set the entrypoint
CMD ["node", "dist/index.js"]

# Usage instructions
# To run with a custom config:
# docker run -v /path/to/config.yaml:/app/config/config.yaml -e PRIVATE_KEY=your_key your-image-name