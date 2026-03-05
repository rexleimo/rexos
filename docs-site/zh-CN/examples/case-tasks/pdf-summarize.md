# PDF 总结（pdf）

**目标：** 从 workspace 内的 PDF 提取文本，并写一份简洁总结。

## 前置条件

- workspace 里有一个 PDF 文件（也可以用下面命令下载一个示例）。
- 已配置 LLM provider（本地测试推荐 Ollama）。

## 下载示例 PDF（可选）

=== "Bash（macOS/Linux）"
    ```bash
    mkdir -p samples
    curl -L -o samples/dummy.pdf https://www.w3.org/WAI/ER/tests/xhtml/testfiles/resources/pdf/dummy.pdf
    ```

=== "PowerShell（Windows）"
    ```powershell
    New-Item -ItemType Directory -Force samples | Out-Null
    Invoke-WebRequest -Uri "https://www.w3.org/WAI/ER/tests/xhtml/testfiles/resources/pdf/dummy.pdf" -OutFile "samples/dummy.pdf"
    ```

## 运行

=== "Bash（macOS/Linux）"
    ```bash
    loopforge agent run --workspace . --prompt "使用 pdf 工具从 samples/dummy.pdf 提取文本（max_pages=10, max_chars=12000）。然后写 notes/pdf_summary.md，包含：(1) 6 条 bullet 总结，(2) 关键词/术语，(3) 你观察到的缺失/乱码/异常段落。只能基于提取到的 PDF 文本，不要编造内容。"
    ```

=== "PowerShell（Windows）"
    ```powershell
    loopforge agent run --workspace . --prompt "使用 pdf 工具从 samples/dummy.pdf 提取文本（max_pages=10, max_chars=12000）。然后写 notes/pdf_summary.md，包含：(1) 6 条 bullet 总结，(2) 关键词/术语，(3) 你观察到的缺失/乱码/异常段落。只能基于提取到的 PDF 文本，不要编造内容。"
    ```

## 可选：选择页码

如果你只需要部分页码，可以传入从 1 开始的选择器，例如 `pages=\"2-3\"`（或 `pages=\"2,4-6\"`）。

## 预期产物

- `notes/pdf_summary.md`
