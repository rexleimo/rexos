# 环境自检

**目标：** 用 2 分钟确认本地配置、路由和工具沙盒都能跑通。

## 步骤

1) 初始化一次：

```bash
rexos init
```

2) 创建一个临时 workspace：

```bash
mkdir -p rexos-demo
cd rexos-demo
```

3) 跑一个最小任务：写文件 + 运行 shell 命令：

```bash
rexos agent run --workspace . --prompt "创建 notes/hello.md（写一句问候）。然后运行 shell 命令 'pwd && ls -la'，把输出保存到 notes/env.txt。最后回复你写入了哪些路径。"
```

## 预期结果

- `notes/hello.md`
- `notes/env.txt`
