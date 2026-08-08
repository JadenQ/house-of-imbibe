import { defineConfig } from "vite";

export default defineConfig({
  server: {
    port: 5173,
    proxy: {
      // 开发模式：API 转发到 Rust 单二进制
      "/api": "http://localhost:8080",
    },
  },
  build: {
    target: "es2020",
  },
});
