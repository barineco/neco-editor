import { defineConfig } from "vite"
import { resolve } from "path"

export default defineConfig({
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    fs: {
      // Allow serving files from the parent directory so that
      // `file:../neco-editor-wasm/pkg` (resolved to absolute paths via
      // symlink) and `file:../neco-editor-ts` can be loaded by the dev server.
      allow: [".."],
    },
  },
  build: {
    rollupOptions: {
      input: resolve(__dirname, "index.html"),
    },
    target: "esnext",
  },
})
