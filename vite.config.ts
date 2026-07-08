import { defineConfig } from "vite";

// Tauri expects a fixed dev port and no auto-opening browser.
export default defineConfig({
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    watch: {
      // Don't let vite's watcher recurse into the Rust build output.
      ignored: ["**/src-tauri/**", "**/crates/**", "**/target/**"],
    },
  },
  build: {
    outDir: "dist",
    target: ["es2022", "chrome105", "safari14"],
    sourcemap: false,
  },
});
