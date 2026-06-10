# mdseek

`mdseek` 是一个用 Rust 编写的本地 Markdown 知识库搜索与分析工具。它可以扫描 Markdown 文件夹，解析标题、链接和标签，建立倒排索引，进行关键词搜索，并分析反向链接、失效链接和孤立文档。

GitHub 仓库：https://github.com/lildayuan/nk-mdseek-rust-final

这个项目适合作为 Rust 课程期末大作业：主要逻辑全部由 Rust 实现，包含模块化设计、错误处理、trait、struct、enum、泛型风格接口、文件系统处理、搜索算法和测试。

## 功能

- 递归扫描 `.md` / `.markdown` / `.mdown` 文件
- 解析 Markdown 标题、`[text](target.md)` 链接、`[[wiki link]]` 链接和 `#tag`
- 建立倒排索引
- 支持关键词搜索、权重排序和命中片段展示
- 支持索引缓存保存与加载
- 分析失效链接、反向链接、标签统计和孤立文档
- 导出 Mermaid 格式知识图谱
- 生成 Markdown / HTML 知识库分析报告
- 主动推荐潜在缺失的笔记链接
- 给知识库健康评分并输出修复建议

## 编译与运行

```bash
cargo build
cargo test
cargo run -- help
```

## 依赖说明

项目没有使用第三方 crate，运行环境只需要 Rust stable 工具链和 Cargo。扫描、解析、索引、搜索、缓存和并发加载均使用 Rust 标准库实现。

使用示例：

```bash
cargo run -- index ./examples/notes
cargo run -- search "ownership borrowing" --root ./examples/notes
cargo run -- stats --root ./examples/notes
cargo run -- links --root ./examples/notes
cargo run -- backlinks ./examples/notes/search.md --root ./examples/notes
cargo run -- graph --root ./examples/notes
cargo run -- report --root ./examples/notes --format html --output report.html
cargo run -- suggest-links --root ./examples/notes
cargo run -- doctor --root ./examples/notes
```

默认情况下，`index` 会在目标目录下生成 `.mdseek-cache`：

```bash
cargo run -- index ./examples/notes --cache ./examples/notes/.mdseek-cache
cargo run -- search "rust" --cache ./examples/notes/.mdseek-cache
```

## 命令

```text
mdseek index <root> [--cache <file>]
mdseek search <query> [--root <dir>] [--cache <file>] [--limit <n>] [--case-sensitive]
mdseek stats [--root <dir>] [--cache <file>]
mdseek links [--root <dir>] [--cache <file>]
mdseek backlinks <file> [--root <dir>] [--cache <file>]
mdseek graph [--root <dir>] [--cache <file>]
mdseek report [--root <dir>] [--cache <file>] [--format markdown|html] [--output <file>]
mdseek suggest-links [--root <dir>] [--cache <file>] [--limit <n>] [--min-score <n>]
mdseek doctor [--root <dir>] [--cache <file>]
```

## 项目结构

```text
src/
  main.rs        程序入口
  lib.rs         库模块导出
  cli.rs         命令行解析与命令执行
  scanner.rs     文件扫描与文档加载
  parser.rs      Markdown 标题、链接、标签解析
  tokenizer.rs   分词器 trait 与简单实现
  index.rs       倒排索引
  search.rs      搜索评分、排序和片段生成
  analyzer.rs    反链、断链、标签、知识图谱分析
  report.rs      Markdown / HTML 报告生成
  storage.rs     缓存保存与加载
  error.rs       统一错误类型
  insights.rs    潜在互链推荐与知识库健康诊断
  types.rs       核心数据结构
tests/
  knowledge_flow.rs 端到端核心流程测试
```

## Rust 特性对应

- ownership / borrowing：文档加载、索引构建、搜索时大量使用所有权转移和借用
- `struct`：`Document`、`SearchIndex`、`Posting`、`KnowledgeReport`
- `enum`：`Field`、`LinkKind`、`Command`、`MdSeekError`
- `trait`：`Tokenizer`
- 错误处理：统一返回 `Result<T, MdSeekError>`，避免在业务代码中大量使用 `unwrap`
- 并发：`scanner.rs` 使用 `std::thread` 和 `std::sync::mpsc` 并行加载 Markdown 文件
- 模块化：扫描、解析、索引、搜索、分析、存储分离
- 测试：包含分词、解析、索引、搜索、分析、缓存和核心流程测试

## 特色功能

`suggest-links` 会分析标题、正文、标签和现有链接，找出“内容高度相关但还没有互链”的笔记关系。它不是简单的全文搜索，而是输出可解释原因，例如共享标签、标题词命中、正文中出现目标标题等。

`doctor` 会根据断链、孤立文档、标签缺失、潜在缺失链接等信号给知识库打分，并输出修复建议。这类主动维护知识库健康的能力，在许多成熟笔记工具中也不是默认内置功能。

`report` 会把统计信息、健康评分、推荐补链、断链列表、孤立文档、反链概览和 Mermaid 图谱汇总成 Markdown 或 HTML 报告，适合课程展示视频中直接打开演示。

## 展示样例

`examples/agent-notes/` 是专门为展示视频准备的 agent 主题样例知识库，包含搜索命中、断链、孤立文档、潜在缺失链接和健康评分等场景。推荐录屏顺序：

```bash
cargo run -- search "工具调用" --root ./examples/agent-notes --limit 3
cargo run -- links --root ./examples/agent-notes
cargo run -- suggest-links --root ./examples/agent-notes --limit 8
cargo run -- doctor --root ./examples/agent-notes
cargo run -- report --root ./examples/agent-notes --format html --output agent-report.html
```

## 后续可扩展方向

- 使用 BM25 公式进一步优化排序
- 支持增量索引，只重新解析变更文件
- 增加 TUI 搜索界面
- 支持更多 Markdown 语法
