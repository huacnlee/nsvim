# NSVim website

Static Astro site using Bun and GitHub Pages. Fonts are bundled locally. English lives at `/nsvim/`, Simplified Chinese at `/nsvim/zh-CN/`.

```sh
cd website
bun install --frozen-lockfile
bun run dev
bun run build
bun x playwright install chromium
bun run test
```

The default base is `/nsvim`. Set `BASE_PATH` and `SITE_URL` together for another host. The production browser tests verify the GitHub Pages path.

`.github/workflows/website.yml` builds and tests pull requests, then deploys `main` using GitHub Pages. Repository Settings → Pages → Source must be **GitHub Actions**.

`public/favicon.svg` is a copy of `assets/editor.svg`: an N on a black tile: a blue left stem, a green right stem and a white diagonal. The header draws the same N in the colours of the page theme. The hero shows `public/screenshot.png`, a link to `assets/screenshot.png`.

## The editor slot

`src/components/EditorDemo.astro` is the editor on the page. Without a live build it shows `src/components/Mock.astro`, a model of the window in HTML that changes mode with `i`, `v`, `:` and `Esc`.

To show the real NSVim, put a static build of it for the web in `public/demo/`, with `index.html` at the top. Use relative paths inside the bundle, because the page serves it at `/nsvim/demo/`. The next build finds `public/demo/index.html` and shows the bundle in a frame. Nothing else changes.

`ext/nsvim-native/web/build.sh` makes this build. It runs `nsh package --target web` and copies the site to `public/demo/`. It needs Rust with the `wasm32-unknown-unknown` target, clang, llvm-ar, the GTK 4 headers on Linux and the NSH CLI 0.7.0. The build is about 28 MB, so Git ignores `public/demo/`. The website workflow builds it before each deploy.

On the web, NSVim opens a demo project that lives in memory (`ext/nsvim-native/web/demo/`). `:w` keeps the text until the page closes. Commands, language servers and the control socket are not on the web. The editor takes the keys only after a click, so the page keeps its own keys.
