import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// The gateway serves the built assets at the same origin as the API. In
// development Vite proxies /api so the client needs no CORS configuration.
export default defineConfig({
  plugins: [svelte()],
  server: {
    proxy: {
      "/api": "http://localhost:8080",
    },
  },
});
