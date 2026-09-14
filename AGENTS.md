# CursorMochi 工程约定

## 目标

Rust + GTK4/GIO 的本地 Linux 光标管理器。当前只做 v0.1：发现、真实 Xcursor 预览、GNOME 显式应用与本会话单级撤销。优先稳定、兼容、正确性、可维护性，不扩大到商店/导入/安装/网络。

## 开始工作前

读取当前仓库、适用的 AGENTS.md、git 状态、`docs/ARCHITECTURE_V2.md`、`docs/IMPLEMENTATION_V2.md`、`docs/ACCEPTANCE_V2.md`。尊重现有用户修改；本文件并不授权覆盖既有规则或重建仓库。

简述实施顺序后继续动手，不停在设计分析。已实现功能先核对，不重复建设。

## 边界

- core/app 无 GTK、GLib、GIO 依赖；platform 实现 app 的窄接口，gtk 组装依赖。
- 扫描、解码和文件读取不进入 GTK 主循环。任务有界，过期结果不得污染新选择。
- 预览读选中主题的真实文件，不用当前系统主题的命名 cursor 冒充。
- 不把同名主题整套目录简单去重；解析结果附来源与诊断。
- 正常本地主题链接可按策略读取；不能无限跟随，更不能执行主题内容。
- 用户输入路径、元数据和图片不可信；分配前检查边界与资源预算。
- 第一方代码禁止 unsafe；不得据此宣称所有依赖都无 unsafe。
- 正常输入错误不使用 unwrap/expect/panic；测试中可以为清楚的断言使用。
- 不为未来功能建立空 crate、通用插件系统、DI 框架、后台 daemon 或多套异步运行时。

## 桌面安全

只有明确点击 Apply/Undo 才能写 settings。读取/启动/预览不修改配置。

先查 schema/key/可写性和会话能力。不支持时只读，不崩溃。保存 effective value 与 user override；只修改用户本次请求的 key。

撤销是 app 保存的快照与冲突策略，不是 GSettings.revert。检测外部修改，默认不覆盖。写入成功、回读一致、真实指针生效分开报告。

普通测试必须注入 fake/memory backend；不得修改用户真实 dconf。不要在 Drop、错误处理或重启时自动恢复桌面。不得自行 sudo、推送、发布、删除用户主题或修改远程设置。

## 验证

建立 workspace/lock 后执行：

```bash
cargo fmt --all --check
cargo test --locked -p cursormochi-core -p cursormochi-app
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --release -p cursormochi-gtk --locked
```

只验证声明支持的 feature 组合，不盲目使用 --all-features。新依赖记录用途、许可、维护状态、native 需求和 MSRV。

## 报告

分开记录自动测试、隔离 GUI、真实 GNOME、缺失环境与未验证项。目标平台不等于实测平台；没有运行的命令不能记 PASS。工具、网络或显示环境缺失时继续可执行的独立任务，不造假结果。

AGENTS.md 是工程指导，不是权限隔离；安全还需依靠后端注入、权限、测试和人工验收。
