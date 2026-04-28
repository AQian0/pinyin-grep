# pinyin-grep

[English](README.md) | [中文](README.zh.md)

检测项目中以**汉语拼音**命名的标识符（变量、函数、类型等）。基于
[ast-grep](https://ast-grep.github.io/) 构建——以 Rust 库的形式**内置**，
不通过外部进程协作——所以匹配结果是 AST 级别的精准定位，而不是正则猜测。

```
$ pinyin-grep src/
src/foo.ts:4:7    high  xinXi
  └─ xin·xi  (2 syllables)
src/foo.ts:7:10   high  huoQuYongHu
  └─ huo·qu·yong·hu  (4 syllables (ambiguous))
     alt: huo=hu·o
src/foo.ts:15:7   high  GuanLiYuan
  └─ guan·li·yuan  (3 syllables (ambiguous))
     alt: guan=gu·an | yuan=yu·an
```

## 为什么需要它

中英混拼的标识符（`huoQuYongHu`、`shujuKu`……）在中文团队的代码里非常常见，
对所有人的阅读体验都是一种损失。`pinyin-grep` 把它们一次性找出来，方便在
重命名、Code Review，或者交给 LLM 自动起一个英文名字时使用。

## 亮点

- **AST 驱动**：`ast-grep-core` + `ast-grep-language` 解析源文件，仅捕获
  你想要的 AST 节点（变量名、函数名、类型名等），不会被注释、字符串误伤。
- **自实现拼音识别**：内置普通话音节表 + 动态规划切分，不依赖任何外部
  字典或服务。
- **两种稳定输出**：NDJSON 给工具 / AI 消费，富文本给人阅读。
- **单一静态二进制**：不需要 Python/Node 运行时，也不需要单独安装
  `ast-grep`。

## 安装

```sh
cargo install --path .
```

当前支持 **TypeScript / TSX / Rust**。新增语言只需在
`src/lang.rs` 和 `src/patterns.rs` 增加少量映射与 pattern。

## 用法

```sh
# 扫描目录（自动遵守 .gitignore，由 ignore crate 提供）
pinyin-grep src/

# 强制指定语言
pinyin-grep --lang ts src/

# 自定义 AST pattern（会替换该语言的内置 pattern 集）
pinyin-grep --lang ts --pattern 'const $NAME = $$$VAL' src/

# 仅诊断单个标识符（不读取文件）
echo "huoQuYongHu\nloadFile\nxinxi" | pinyin-grep --names --min-confidence low

# 输出 NDJSON 接到 jq / AI 工具
pinyin-grep src/ --format ndjson | jq '.identifier'
```

### 常用参数

| 参数 | 作用 |
| --- | --- |
| `--lang <ts\|tsx\|rs>` | 强制指定语言（不指定则按扩展名推断）。 |
| `--pattern <PATTERN>` | 覆盖内置 pattern 集，可重复使用，需要配合 `--lang`。 |
| `--meta-var <NAME>` | 从 pattern 的哪个元变量中取标识符，默认 `NAME`。 |
| `--names` | 从 stdin 每行读一个标识符，跳过文件 IO。 |
| `--format <auto\|text\|ndjson>` | 输出格式。`auto` = 管道输出 NDJSON，TTY 输出文本。 |
| `--min-confidence <low\|medium\|high>` | 置信度阈值，默认 `medium`。 |
| `--ignore <REGEX>` | 跳过匹配该正则的标识符，可重复使用。 |
| `--show-all` | 忽略置信度阈值，全部展示。 |

## 内置 pattern

不传 `--pattern` 时，按文件语言应用以下 ast-grep pattern：

**TypeScript / TSX**
- `const $NAME = $$$`
- `let $NAME = $$$`
- `var $NAME = $$$`
- `function $NAME($$$) { $$$ }`
- `class $NAME { $$$ }`
- `interface $NAME { $$$ }`
- `type $NAME = $$$`
- `enum $NAME { $$$ }`

**Rust**
- `fn $NAME($$$) { $$$ }`
- `fn $NAME($$$) -> $RET { $$$ }`
- `struct $NAME { $$$ }` / `struct $NAME($$$);` / `struct $NAME;`
- `enum $NAME { $$$ }`
- `trait $NAME { $$$ }`
- `let $NAME = $$$` / `let mut $NAME = $$$`
- `const $NAME: $T = $$$` / `static $NAME: $T = $$$`
- `type $NAME = $$$`

每条 pattern 的 `$NAME` 元变量就是工具会去检测的标识符。

## 输出 schema（NDJSON）

每行都是一个自包含的 JSON 对象。行号、列号是 **0-based**，与 ast-grep
官方 JSON 输出保持一致。

```json
{
  "file": "src/foo.ts",
  "range": {
    "start": { "line": 6, "column": 9 },
    "end":   { "line": 6, "column": 20 }
  },
  "identifier": "huoQuYongHu",
  "tokens": [
    { "text": "huo",  "syllables": [["huo"], ["hu", "o"]] },
    { "text": "qu",   "syllables": [["qu"]] },
    { "text": "yong", "syllables": [["yong"]] },
    { "text": "hu",   "syllables": [["hu"]] }
  ],
  "score": 9,
  "confidence": "high",
  "ambiguous": true
}
```

字段保证：

- `tokens[*].syllables` 按音节数升序排列，因此 `syllables[0]` 即为
  **首选切分**（最长匹配优先）。
- `confidence` 取值为 `low` / `medium` / `high` 之一。
- 任意 token 存在多种切分时（例如 `xian` = `xian` 或 `xi'an`），
  `ambiguous` 为 `true`。
- `--names` 模式下不含 `file` 与 `range` 字段。

## 拼音识别原理

1. **标识符分词。** `huoQuYongHu` 被切成 `[huo, qu, yong, hu]`，处理
   camelCase / snake_case / kebab-case，并对 `URLParser` → `[url, parser]`
   这种缩写边界做特殊处理。
2. **音节切分。** 每个 token 小写化后送入动态规划切分器，对照
   `src/syllables.rs` 中硬编码的普通话音节集枚举所有合法切分。
3. **置信度评分。** 由若干加分/减分规则组合：
   - 每个成功切分的 token `+N`
   - 当**所有** token 都是有效拼音，且标识符存在显式的词边界时给加分
   - 当只有部分 token 是拼音、且平均音节长度偏短时减分（避免
     `userName` 这类英文混合误报）
   - 单 token 标识符若刚好命中常见英文短词黑名单（`me`、`men`、
     `pin` 等）小幅扣分。

默认参数下 `--min-confidence medium` 能稳定识别中国开发者常见的拼音命名
（`huoQu`、`xinXi`、`huoQuYongHuXinXi`、`GuanLiYuan`、`SHU_LIANG`），
同时过滤掉以英文为主的命名（`userName`、`loadFile`、`getUser`）。

## 局限性

- v1 仅支持 TypeScript / TSX / Rust。新增语言只是 `src/lang.rs` 与
  `src/patterns.rs` 的少量改动。
- 单 token 标识符天然有歧义，`xinxi` 会落到 `low` 置信度，默认会被过滤；
  如果你需要看到，加 `--min-confidence low`。
- 不识别声调：`huo` 会同时对应 火 / 或 等，这对标识符级别的检测足够。
- 部分音节也是英文常见词（`men`、`pan`、`die`……），单 token 形态下
  会触发黑名单减分。

## 开发

```sh
cargo build           # 构建
cargo test            # 40 个单元 + 6 个集成测试
cargo clippy --all-targets -- -D warnings
cargo run -- tests/samples/   # 在样例上做冒烟测试
```
