import { existsSync } from 'node:fs';
import { expect, test } from '@playwright/test';

const shellCommand = 'curl -fsSL https://huacnlee.github.io/nsvim/install.sh | sh';
const windowsCommand = 'irm https://huacnlee.github.io/nsvim/install.ps1 | iex';
const sourceCommand = 'git clone https://github.com/huacnlee/nsvim && cd nsvim && cargo build --release --locked && nsh build --release -o target/release/nsh';
const live = existsSync(new URL('../public/demo/index.html', import.meta.url));

test('English and Chinese routes keep correct links, language and metadata', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.goto('./');
  await expect(page.locator('h1')).toHaveText('Vim keys. Native windows.');
  await expect(page.locator('.eyebrow')).toHaveText('Next Simple VIM · Native Shell VIM');
  await expect(page.locator('link[rel="canonical"]')).toHaveAttribute('href', 'https://huacnlee.github.io/nsvim/');
  await expect(page.locator('link[rel="icon"]')).toHaveAttribute('href', '/nsvim/favicon.svg');
  const favicon = await page.request.get('/nsvim/favicon.svg');
  expect(favicon.ok()).toBe(true);
  expect(favicon.headers()['content-type']).toContain('image/svg+xml');
  await expect(page.getByRole('link', { name: 'Native Shell' })).toHaveAttribute('href', 'https://nsh.dev');
  await expect(page.locator('footer').getByRole('link', { name: 'bjarneo/editor' })).toHaveAttribute('href', 'https://github.com/bjarneo/editor');

  await page.locator('#language-toggle').click();
  await page.getByRole('menuitemradio', { name: '简体中文' }).click();
  await expect(page.locator('html')).toHaveAttribute('lang', 'zh-CN');
  await expect(page.locator('h1')).toHaveText('Vim 的按键原生的窗口');
  await expect(page.locator('link[rel="canonical"]')).toHaveAttribute('href', 'https://huacnlee.github.io/nsvim/zh-CN/');
  await expect(page.getByRole('link', { name: 'GitHub', exact: true })).toHaveAttribute('href', 'https://github.com/huacnlee/nsvim');
  await expect(page.locator('#command-linux')).toHaveText(shellCommand);
  await expect(page.locator('#command-windows')).toHaveText(windowsCommand);
  await expect(page.locator('#command-source')).toHaveText(sourceCommand);
  expect(errors).toEqual([]);
});

test('the origin section credits the original authors in both languages', async ({ page }) => {
  for (const route of ['./', './zh-CN/']) {
    await page.goto(route);
    const origin = page.locator('#origin');
    await expect(origin).toContainText('Bjarne Oeverli');
    await expect(origin.getByRole('link', { name: 'bjarneo/editor ↗' })).toHaveAttribute('href', 'https://github.com/bjarneo/editor');
    await expect(origin.getByRole('link', { name: 'LICENSE ↗' })).toHaveAttribute('href', 'https://github.com/huacnlee/nsvim/blob/main/LICENSE');
  }
});

test('the header draws the N with its left stroke in the brand color', async ({ page }) => {
  for (const [colorScheme, fill] of [['dark', 'rgb(108, 158, 255)'], ['light', 'rgb(47, 95, 208)']] as const) {
    await page.emulateMedia({ colorScheme });
    await page.goto('./');
    await expect(page.locator('.mark-legend')).toHaveCount(0);
    await expect(page.locator('.brand-mark line')).toHaveCount(4);
    await expect(page.locator('.brand-mark line').first()).toHaveCSS('stroke', fill);
  }
});

test('the editor slot changes mode with keys and buttons', async ({ page }) => {
  test.skip(live, 'the live editor takes its own keys');
  await page.goto('./');
  const mock = page.locator('.editor-demo .mock');
  await expect(mock).toHaveAttribute('data-mode', 'normal');
  await page.locator('body').press('i');
  await expect(mock).toHaveAttribute('data-mode', 'insert');
  await expect(page.locator('#mock-pill')).toHaveText('INSERT');
  await page.getByRole('button', { name: /VISUAL/ }).click();
  await expect(mock).toHaveAttribute('data-mode', 'visual');
  await page.locator('body').press(':');
  await expect(mock).toHaveAttribute('data-mode', 'command');
  await page.locator('body').press('Escape');
  await expect(mock).toHaveAttribute('data-mode', 'normal');
  await expect(page.getByRole('button', { name: /NORMAL/ })).toHaveAttribute('aria-pressed', 'true');
});

test('the editor slot shows the live build when public/demo has one', async ({ page }) => {
  test.skip(!live, 'no live build in public/demo');
  await page.goto('./');
  await expect(page.locator('.editor-demo iframe')).toHaveAttribute('src', '/nsvim/demo/');
});

test('related projects link to the other projects in both languages', async ({ page }) => {
  for (const [route, name] of [['./', 'More projects'], ['./zh-CN/', '更多项目']]) {
    await page.goto(route);
    const related = page.getByRole('region', { name });
    await expect(related.getByRole('link', { name: 'Disktree' })).toHaveAttribute('href', 'https://huacnlee.github.io/disktree/');
    await expect(related.getByRole('link', { name: 'Omasend' })).toHaveAttribute('href', 'https://huacnlee.github.io/omasend/');
    await expect(related.getByRole('link', { name: 'Omamail' })).toHaveAttribute('href', 'https://huacnlee.github.io/omamail/');
    await expect(related.getByRole('link', { name: 'Noop' })).toHaveAttribute('href', 'https://huacnlee.github.io/noop/');
    await expect(related.getByRole('link', { name: 'huacnlee/cargo' })).toHaveAttribute('href', 'https://huacnlee.github.io/cargo/');
    await expect(related.getByRole('link', { name: 'omarchy-which-key' })).toHaveAttribute('href', 'https://github.com/huacnlee/omarchy-which-key');
    await expect(related.locator('li p')).toHaveCount(6);
  }
});

test('light and dark toggle follows system then persists across reload and language navigation', async ({ page }) => {
  await page.emulateMedia({ colorScheme: 'light' });
  await page.goto('./');
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');
  await expect(page.getByRole('button', { name: 'Dark mode' })).toHaveAttribute('aria-pressed', 'false');
  await page.emulateMedia({ colorScheme: 'dark' });
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
  await page.getByRole('button', { name: 'Dark mode' }).click();
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');
  await page.reload();
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');
  await page.locator('#language-toggle').click();
  await page.getByRole('menuitemradio', { name: '简体中文' }).click();
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');
  await page.getByRole('button', { name: '深色模式' }).click();
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
});

test('language menu handles keyboard selection, escape and outside click', async ({ page }) => {
  await page.goto('./');
  await page.getByRole('button', { name: 'Language' }).focus();
  await page.keyboard.press('ArrowDown');
  await expect(page.getByRole('menuitemradio', { name: 'English' })).toBeFocused();
  await expect(page.getByRole('menuitemradio', { name: 'English' })).toHaveAttribute('aria-checked', 'true');
  await page.keyboard.press('End');
  await expect(page.getByRole('menuitemradio', { name: '简体中文' })).toBeFocused();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('button', { name: 'Language' })).toBeFocused();
  await expect(page.getByRole('menu')).toHaveCount(0);
  await page.getByRole('button', { name: 'Language' }).click();
  await page.locator('h1').click();
  await expect(page.getByRole('menu')).toHaveCount(0);
});

test('tabs switch with mouse and keyboard and copy only the active command', async ({ page, context }) => {
  await context.grantPermissions(['clipboard-read', 'clipboard-write']);
  await page.goto('./');
  const unix = page.getByRole('tab', { name: 'macOS / Linux', exact: true });
  const windows = page.getByRole('tab', { name: 'Windows', exact: true });
  const source = page.getByRole('tab', { name: 'From source', exact: true });
  await expect(unix).toHaveAttribute('aria-selected', 'true');
  await expect(page.getByRole('tabpanel')).toHaveCount(1);
  await unix.focus();
  await page.keyboard.press('ArrowRight');
  await expect(windows).toBeFocused();
  await expect(page.getByRole('tabpanel')).toHaveAttribute('id', 'panel-windows');
  await page.getByRole('button', { name: 'Copy', exact: true }).click();
  await expect(page.getByRole('status')).toHaveText('Copied');
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(windowsCommand);
  await windows.focus();
  await page.keyboard.press('End');
  await expect(source).toBeFocused();
  await page.keyboard.press('Home');
  await expect(unix).toBeFocused();
  await page.keyboard.press('ArrowLeft');
  await expect(source).toBeFocused();
  await unix.click();
  await page.getByRole('button', { name: 'Copy', exact: true }).click();
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(shellCommand);
});

test('the installers are served next to the pages', async ({ page }) => {
  await page.goto('./');
  const shell = await page.request.get('/nsvim/install.sh');
  expect(shell.ok()).toBe(true);
  expect(await shell.text()).toContain('NSVIM_INSTALL_DIR');
  const powershell = await page.request.get('/nsvim/install.ps1');
  expect(powershell.ok()).toBe(true);
  expect(await powershell.text()).toContain('NSVIM_INSTALL_DIR');
  await expect(page.locator('#panel-linux .script-link')).toHaveAttribute('href', 'https://github.com/huacnlee/nsvim/blob/main/scripts/install.sh');
  await page.goto('./zh-CN/');
  await expect(page.getByRole('tab', { name: '从源码构建', exact: true })).toBeVisible();
  await expect(page.locator('#panel-linux .script-link')).toHaveText('查看安装脚本 ↗');
});

for (const [width, height] of [[360, 640], [1440, 900]]) {
  test(`installation follows hero in first viewport at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height });
    for (const route of ['./', './zh-CN/']) {
      await page.goto(route);
      expect(await page.locator('.hero').evaluate(element => element.nextElementSibling?.id)).toBe('install');
      const bounds = await page.locator('#command-linux').boundingBox();
      expect(bounds).not.toBeNull();
      expect(bounds!.y + bounds!.height).toBeLessThanOrEqual(height);
    }
  });
}

for (const width of [360, 768, 1440]) {
  test(`layout fits ${width}px and reduced motion retains content`, async ({ page }) => {
    await page.setViewportSize({ width, height: 1000 });
    await page.emulateMedia({ reducedMotion: 'reduce' });
    for (const route of ['./', './zh-CN/']) {
      await page.goto(route);
      expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width);
      await expect(page.locator('h1')).toBeVisible();
      if (width > 680) {
        await expect(page.locator('.shot img')).toBeVisible();
      }
    }
  });
}

test('keyboard reaches skip link and section link', async ({ page }) => {
  await page.goto('./');
  await page.keyboard.press('Tab');
  await expect(page.getByRole('link', { name: 'Skip to content' })).toBeFocused();
  await page.keyboard.press('Enter');
  await page.keyboard.press('Tab');
  await expect(page.getByRole('link', { name: 'See how it works' })).toBeFocused();
  await page.keyboard.press('Enter');
  await expect(page).toHaveURL(/#workflow$/);
});

test('Chinese headings have no trailing full stop', async ({ page }) => {
  await page.goto('./zh-CN/');
  for (const text of await page.locator('h1, h2, h3, .eyebrow').allTextContents()) {
    expect(text.trim()).not.toMatch(/。/);
  }
});

test('every page asks for sponsorship with a visible button', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  for (const [route, text, nav, button] of [['./', 'If this helps you, please sponsor me.', 'Sponsor me', 'Sponsor on GitHub'], ['./zh-CN/', '如果这个项目帮到了你，请赞助我。', '赞助作者', '在 GitHub 上赞助']]) {
    await page.goto(route);
    await expect(page.locator('.sponsor p')).toHaveText(text);
    await expect(page.locator('.sponsor .sponsor-button')).toHaveText(button);
    await expect(page.locator('nav .sponsor-button')).toHaveText(nav);
    await expect(page.locator('nav .sponsor-button')).toBeVisible();
    for (const link of await page.locator('.sponsor-button').all()) {
      await expect(link).toHaveAttribute('href', 'https://github.com/sponsors/huacnlee');
      await expect(link).toHaveAttribute('target', '_blank');
    }
  }
});

test('the sponsor card follows install and stays on phones', async ({ page }) => {
  await page.setViewportSize({ width: 360, height: 640 });
  await page.goto('./');
  await expect(page.locator('#install + .sponsor')).toBeVisible();
  await expect(page.locator('nav .sponsor-button')).toBeHidden();
});
