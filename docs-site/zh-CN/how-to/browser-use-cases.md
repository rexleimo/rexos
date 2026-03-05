# 浏览器案例（配方）

这里把浏览器配方拆成多个独立页面，方便你在侧边栏快速点开查看并复制粘贴。

另见：[浏览器自动化（CDP）](browser-automation.md)。

## 前置条件（默认：CDP）

- 安装任意 Chromium 系浏览器（Chrome/Chromium/Edge）。
- 如果 LoopForge 找不到可执行文件，可以设置 `REXOS_BROWSER_CHROME_PATH`。

可选：使用 Playwright（legacy）后端：设置 `REXOS_BROWSER_BACKEND=playwright`，并按 [浏览器自动化](browser-automation.md) 的步骤安装 Playwright。

## 配方列表

- [有界面 smoke check（example.com）](browser-use-cases/gui-smoke-check.md)
- [更接近真实场景：百度“今天天气”（Browser + Ollama）](browser-use-cases/baidu-weather.md)
- [Wikipedia：打开 → 总结 → 截图](browser-use-cases/wikipedia-summary.md)
- [（从源码）运行浏览器 + Ollama smoke test](browser-use-cases/smoke-test.md)

## 小技巧

- 对搜索引擎来说，直接打开**结果页 URL** 通常更稳（比在首页输入框里打字更不容易被拦）。
- 出错时也尽量在最后调用 `browser_close`。
- 未经用户明确确认，不要输入账号密码，也不要进行任何付费/下单操作。
