# Agent 笔记展示脚本

这组样例文件用于录制 `mdseek` 项目展示视频。

推荐展示顺序：

```bash
cargo run -- search "工具调用" --root ./examples/agent-notes --limit 3
cargo run -- links --root ./examples/agent-notes
cargo run -- suggest-links --root ./examples/agent-notes --limit 8
cargo run -- doctor --root ./examples/agent-notes
cargo run -- report --root ./examples/agent-notes --format html --output agent-report.html
```

这组笔记故意包含：

- 一个断链：`legacy-tool-protocol.md`
- 多个没有手动链接但语义相关的 agent 概念
- 若干孤立文档
- 共享标签，例如 `#智能体`、`#规划`、`#记忆`、`#评估`

#智能体 #展示 #脚本
