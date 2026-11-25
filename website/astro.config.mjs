import { defineConfig } from "astro/config";

// GitHub Pages serves the website at https://huacnlee.github.io/nsvim/.
export default defineConfig({
  site: "https://huacnlee.github.io",
  base: "/nsvim",
  output: "static",
  build: { inlineStylesheets: "always" },
});
