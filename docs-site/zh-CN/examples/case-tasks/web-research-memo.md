# 网页资料调研 Memo（可复制粘贴）

**目标：** 搜索网页 → 抓取 2～3 个来源 → 写一份带 URL 的简短 memo。

## 运行

把主题改成你的问题，然后执行：

=== "macOS/Linux"
    ```bash
    loopforge agent run --workspace . --prompt "主题：'长任务 agent 的有效 harness 设计'。先用 web_search 找 5 个相关来源；挑最靠谱的 3 个用 web_fetch 抓取正文；写 notes/research.md，包含：(1) 5 条要点总结 (2) 3 个关键结论 (3) 仍未解决的问题 (4) 来源 URL 列表。全文不超过 500 字。"
    ```

=== "Windows (PowerShell)"
    ```powershell
    loopforge agent run --workspace . --prompt "主题：'长任务 agent 的有效 harness 设计'。先用 web_search 找 5 个相关来源；挑最靠谱的 3 个用 web_fetch 抓取正文；写 notes/research.md，包含：(1) 5 条要点总结 (2) 3 个关键结论 (3) 仍未解决的问题 (4) 来源 URL 列表。全文不超过 500 字。"
    ```

## 预期产物

- `notes/research.md`

!!! note "遇到 JS-heavy 站点"
    如果 `web_fetch` 抓不到内容，切换到浏览器工具并截图作为证据。

