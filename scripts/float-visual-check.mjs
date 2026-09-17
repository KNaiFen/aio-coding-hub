// GitHub Actions installs Playwright in an isolated runner directory.
import "./require-github-actions.mjs";
import { createServer } from "node:http";
import { readFile, mkdir } from "node:fs/promises";
import { resolve, extname } from "node:path";
import { pathToFileURL } from "node:url";
import assert from "node:assert/strict";

const { chromium } = await import(process.env.AIO_PLAYWRIGHT_MODULE ? pathToFileURL(process.env.AIO_PLAYWRIGHT_MODULE).href : "playwright");
const root = resolve("src-tauri/crates/aio-float/web");
const output = process.env.AIO_VISUAL_OUTPUT || "output/playwright/float";
await mkdir(output, { recursive: true });
const server = createServer(async (request, response) => {
  try {
    const path = resolve(root, `.${new URL(request.url, "http://localhost").pathname}`);
    if (!path.startsWith(`${root}/`) && !path.startsWith(`${root}\\`)) { response.writeHead(403).end(); return; }
    const mime = { ".js": "text/javascript", ".css": "text/css", ".html": "text/html", ".ttf": "font/ttf", ".svg": "image/svg+xml", ".png": "image/png" }[extname(path)];
    const content = await readFile(path);
    response.writeHead(200, { "Content-Type": mime || "application/octet-stream" });
    response.end(content);
  } catch { response.writeHead(404).end(); }
});
await new Promise((done) => server.listen(0, "127.0.0.1", done));
const browser = await chromium.launch({ executablePath: process.env.AIO_CHROME_PATH || undefined, headless: true });
try {
  const errors = [];
  for (const [width, height, scale] of [[280,640,1], [160,300,1], [640,480,2]]) {
    const page = await browser.newPage({ viewport: { width, height }, deviceScaleFactor: scale });
    page.on("pageerror", (error) => errors.push(error.message));
    await page.addInitScript(() => {
      const config = { ip: "127.0.0.1", port: 13799, fontSize: 12, background: "#272B33", opacity: .35, alwaysOnTop: true, clickThrough: false };
      const cells = [];
      window.floatCalls = [];
      window.floatError = null;
      for (const [y, text, fg] of [[0,"并发 0 | 首选 正价PRO20X", "#69aab3"], [1,"Codex | 今日 $187.51", "#71ae7e"], [3,"200 成功 1分钟前", "#71ae7e"], [4,"Codex / gpt-6-astra-xhigh", "#7da0c4"], [5,"正价PRO20X VIBE 无目录", "#b689be"], [6,"直连 5.0s 18.4 t/s", "#69aab3"]]) {
        let x = 0;
        for (const char of text) { const width = char.codePointAt(0) > 255 ? 2 : 1; cells.push({x,y,text:char,width,fg,bg:null,bold:false,italic:false,underline:false}); x += width; }
      }
      window.__TAURI__ = { core: { invoke: async (command, args) => {
        window.floatCalls.push({command, args});
        if (command === "float_frame") return { ...args, cells: cells.filter(cell => cell.x + cell.width <= args.columns && cell.y < args.rows), config, connected: true, error: window.floatError };
        if (command === "float_settings") return { config, hasToken: true, error: null };
        if (command === "float_connect" && window.connectionFailure) throw window.connectionFailure;
        if (command === "float_appearance") Object.assign(config, args);
      } }, window: { getCurrentWindow: () => ({startDragging: async()=>{},startResizeDragging: async()=>{}}) }, event: {listen: async()=>{}} };
    });
    await page.goto(`http://127.0.0.1:${server.address().port}/index.html`);
    await page.waitForFunction(() => document.querySelector("canvas").width > 1 && document.fonts.check("12px Cascadia"));
    await page.waitForTimeout(500);
    const pixels = await page.evaluate(() => {
      const canvas = document.querySelector("canvas");
      const data = canvas.getContext("2d").getImageData(0,0,canvas.width,canvas.height).data;
      let opaque = 0, translucent = 0;
      for(let i=3;i<data.length;i+=4) { if(data[i]===255) opaque++; if(data[i]>50 && data[i]<150) translucent++; }
      return {opaque,translucent,overflow:document.documentElement.scrollWidth>innerWidth};
    });
    assert(pixels.opaque > 20, "text must remain opaque");
    assert(pixels.translucent > 1000, "background must be translucent");
    assert(!pixels.overflow, "dashboard must not overflow");
    const border = await page.evaluate(() => {
      const canvas = document.querySelector('canvas');
      const context = canvas.getContext('2d');
      return {
        edgeAlpha: context.getImageData(0, Math.floor(canvas.height / 2), 1, 1).data[3],
        innerAlpha: context.getImageData(Math.floor(3 * devicePixelRatio), Math.floor(canvas.height / 2), 1, 1).data[3],
      };
    });
    assert(border.edgeAlpha > border.innerAlpha && border.edgeAlpha < 200, 'border must be subtle and visible without changing content opacity');
    await page.screenshot({ path: `${output}/dashboard-${width}-${scale}x.png`, omitBackground: true });
    for (const error of ['认证失败，请更新访问令牌', '连接失败：无法访问提供数据的电脑，请检查 IP 地址、端口和局域网连接']) {
      await page.evaluate(error => { window.floatError = error; }, error);
      await page.waitForFunction(error => {
        const message = document.querySelector('#message');
        const frame = window.floatCalls.filter(call => call.command === 'float_frame').at(-1);
        return message.textContent === error && 6 + frame.args.rows * 12 * 1.2 <= message.getBoundingClientRect().top;
      }, error);
    }
    await page.screenshot({ path: `${output}/offline-${width}-${scale}x.png`, omitBackground: true });
    await page.evaluate(() => { window.floatError = null; });
    await page.waitForFunction(() => !document.querySelector('#message').textContent);
    await page.keyboard.press('Control+=');
    await page.waitForFunction(() => window.floatCalls.some(call => call.command === 'float_appearance' && call.args.fontSize === 13));
    await page.keyboard.press('Tab');
    await page.waitForFunction(() => window.floatCalls.some(call => call.command === 'float_key' && call.args.key === 'Tab'));
    await page.mouse.move(width / 2, height / 2);
    await page.mouse.wheel(0, 100);
    await page.waitForFunction(() => window.floatCalls.some(call => call.command === 'float_key' && call.args.key === 'ArrowDown'));
    await page.mouse.click(width / 2, height / 2, {button:'right'});
    await page.waitForFunction(() => window.floatCalls.some(call => call.command === 'float_menu'));
    for (const fontSize of [8, 32]) {
      await page.evaluate(fontSize => window.__TAURI__.core.invoke('float_appearance', {fontSize}), fontSize);
      await page.waitForFunction(fontSize => {
        const columns = Math.min(400, Math.max(1, Math.floor((innerWidth - 12) / (fontSize * .6))));
        return window.floatCalls.some(call => call.command === 'float_frame' && call.args.columns === columns);
      }, fontSize);
      await page.waitForTimeout(250);
      assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
      await page.screenshot({path:`${output}/dashboard-${width}-${scale}x-font${fontSize}.png`, omitBackground:true});
    }
    const settingsWidth = width === 640 ? 420 : 340;
    await page.setViewportSize({width:settingsWidth,height:600});
    await page.goto(`http://127.0.0.1:${server.address().port}/settings.html`);
    await page.getByRole('tab', {name:'外观', selected:true}).waitFor();
    await page.getByLabel('字号', {exact:true}).fill('12');
    await page.getByRole('button', {name:'增大字号'}).click();
    assert.equal(await page.getByLabel('字号', {exact:true}).inputValue(), '13');
    await page.getByRole('button', {name:'减小字号'}).click();
    await page.getByRole('button', {name:'松绿', exact:true}).click();
    assert.equal(await page.getByLabel('背景颜色', {exact:true}).inputValue(), '#20332e');
    await page.getByRole('button', {name:'石墨', exact:true}).click();
    await page.getByLabel('背景不透明度', {exact:true}).focus();
    await page.keyboard.press('End');
    for (let step = 0; step < 20; step++) await page.keyboard.press('ArrowLeft');
    await page.waitForFunction(() => document.querySelector('#preview').width > 1);
    const previewPixels = await page.locator('#preview').evaluate(canvas => {
      const pixels = canvas.getContext('2d').getImageData(0, 0, canvas.width, canvas.height).data;
      return pixels.filter((value, index) => index % 4 === 3 && value > 200).length;
    });
    assert(previewPixels > 100, 'appearance preview must render');
    assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
    assert(await page.getByRole('button', {name:'应用外观'}).evaluate(button => button.getBoundingClientRect().bottom <= innerHeight), 'default appearance controls fit the window');
    await page.screenshot({path:`${output}/settings-appearance-${settingsWidth}-${scale}x.png`, fullPage:true});
    await page.getByRole('tab', {name:'外观'}).focus();
    await page.keyboard.press('ArrowLeft');
    await page.getByRole('tab', {name:'连接', selected:true}).waitFor();
    await page.getByLabel('访问令牌', {exact:true}).fill('example-token-for-visual-check');
    await page.getByRole('button', {name:'显示令牌'}).click();
    assert.equal(await page.getByLabel('访问令牌', {exact:true}).getAttribute('type'), 'text');
    await page.getByRole('button', {name:'隐藏令牌'}).click();
    await page.getByLabel("IP 地址").fill("192.168.1.2");
    await page.evaluate(() => { window.connectionFailure = '认证失败，请检查访问令牌'; });
    await page.getByRole("button", {name:"测试并保存连接"}).click();
    await page.getByText('认证失败，请检查访问令牌', {exact:true}).waitFor();
    assert(await page.getByRole('button', {name:'测试并保存连接'}).isEnabled());
    await page.evaluate(() => { window.connectionFailure = null; });
    await page.getByRole("button", {name:"测试并保存连接"}).click();
    await page.getByText("已连接", {exact:true}).waitFor();
    assert.equal(await page.getByLabel('访问令牌', {exact:true}).inputValue(), '');
    await page.screenshot({path:`${output}/settings-connection-${settingsWidth}-${scale}x.png`, fullPage:true});
    await page.getByRole('tab', {name:'外观'}).click();
    await page.getByLabel('字号', {exact:true}).fill('32');
    assert(await page.getByRole('button', {name:'增大字号'}).isDisabled());
    await page.getByRole('button', {name:'应用外观'}).click();
    await page.getByText('已保存', {exact:true}).waitFor();
    assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
    await page.screenshot({ path: `${output}/settings-large-font-${settingsWidth}-${scale}x.png`, fullPage:true });
    await page.close();
  }
  assert.deepEqual(errors, []);
  console.log("Float visual checks passed: narrow/wide grids, DPI, font, alpha and settings.");
} finally { await browser.close(); await new Promise((done) => server.close(done)); }
