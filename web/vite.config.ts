import { defineConfig } from "vite";

export default defineConfig({
  server: {
    port: 5173,
    proxy: {
      // 开发模式：API + WS 转发到 Rust 单二进制
      "/api": "http://localhost:8080",
      "/ws": { target: "ws://localhost:8080", ws: true },
    },
  },
  build: {
    target: "es2020",
  },
});
