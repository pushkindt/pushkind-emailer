import { resolve } from "node:path";

import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

export default defineConfig({
  base: "/assets/dist/",
  plugins: [react()],
  test: {
    environment: "jsdom",
    environmentOptions: {
      jsdom: {
        url: "http://localhost/",
      },
    },
    include: ["src/**/*.test.ts?(x)"],
  },
  resolve: {
    dedupe: ["react", "react-dom"],
    alias: [
      { find: "react", replacement: resolve(__dirname, "node_modules/react") },
      {
        find: "react-dom",
        replacement: resolve(__dirname, "node_modules/react-dom"),
      },
    ],
  },
  build: {
    manifest: "manifest.json",
    outDir: resolve(__dirname, "../assets/dist"),
    emptyOutDir: true,
    rollupOptions: {
      input: {
        "app/index.html": resolve(__dirname, "app/index.html"),
        "app/groups.html": resolve(__dirname, "app/groups.html"),
        "app/recipients.html": resolve(__dirname, "app/recipients.html"),
        "app/no-access.html": resolve(__dirname, "app/no-access.html"),
        "app/settings.html": resolve(__dirname, "app/settings.html"),
        "app/unsubscribed.html": resolve(__dirname, "app/unsubscribed.html"),
        "app/history.html": resolve(__dirname, "app/history.html"),
      },
      output: {
        entryFileNames: "entries/[name]-[hash].js",
        chunkFileNames: "chunks/[name]-[hash].js",
        assetFileNames: ({ name }) => {
          if (name?.endsWith(".css")) {
            return "styles/[name]-[hash][extname]";
          }

          return "assets/[name]-[hash][extname]";
        },
      },
    },
  },
});
