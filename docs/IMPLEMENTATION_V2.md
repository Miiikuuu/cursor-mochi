# CursorMochi v0.1 — 实施任务书 v2

本文件要求实际实施，不是只输出计划。先读取仓库，再按以下任务推进。架构解释见 `ARCHITECTURE_V2.md`，回归门槛见 `ACCEPTANCE_V2.md`。

## 任务顺序

```text
M0 环境/依赖/现状核对
 → M1 四层骨架与无 GUI 测试
 → M2 主题发现、来源解析、受控 Xcursor 解码
 → M3 桌面能力与隔离设置接口
 → M4 只读真实预览窗口
 → M5 显式应用、冲突检测、单级撤销
 → M6 回归、诊断、构建说明与验收记录
```

保持小步提交的粒度，但除非用户明确授权，不自动推送、发版或修改远程仓库配置。已完成的任务核实后跳过，不重复实现。

## M0：核对环境，不修改桌面

先读适用 AGENTS.md、当前 git 状态、已有 Cargo 工作区和代码。保存用户未提交修改，不重建整个仓库。

检查可用的 Rust/Cargo、GTK/GLib/GIO 开发库、实际会话类型、当前图形后端、原生 GUI 可用性。示例只读命令：

```bash
rustc --version
cargo --version
pkg-config --modversion gtk4
pkg-config --modversion glib-2.0 gio-2.0
printf '%s\n' "$XDG_CURRENT_DESKTOP" "$XDG_SESSION_TYPE"
```

分别记录“用户目标环境”和“当前执行环境”。容器能编译不代表拥有用户 GNOME 会话。不要推测 Ubuntu 版本对应某个 GTK 小版本；不要把 GNOME 50 和 GTK 50 混为一谈。

交付 `docs/ENVIRONMENT.md`，内容包括：实际探测值、缺失依赖、Rust/MSRV、gtk-rs 与 native runtime 下限、feature 选择、解析器候选与依赖许可/维护检查、Xcursor 路径/继承策略依据。

不凭文档示例选择“最新版本”，不自动执行 sudo 安装。缺少 GUI 环境时继续 core/app 的可执行工作；GUI 验证明确标为 blocked/unverified，不造假截图。

**完成条件：**版本与路径假设可核对；本阶段未写入用户光标设置。

## M1：最小四层工作区

建立 core、app、platform、gtk 四个 crate，按架构依赖方向组织。已有模块结构合理时做最小改造。

先确定 ThemeName、ThemeRecord、ResolutionTrace、CursorVariant/Frame、SettingsSnapshot、ChangeIntent、ChangeReceipt、Diagnostic 的语义。仅实现当前流程需要的字段；不要预建商店 schema 或十几层 trait。

core/app 的测试不应需要安装 GTK 原生库、读取 HOME 或连接 D-Bus。第一方代码禁止 unsafe，正常失败使用 Result，不用 unwrap/expect/panic。

生成并提交应用的 Cargo.lock；确定验证过的 toolchain/MSRV 后再固定版本。添加检查依赖方向的测试或 CI 命令说明，确保 core/app 不偷偷引入 GUI 依赖。

**完成条件：**核心测试能独立执行，工作区无依赖环，GUI crate 能构建基础窗口（若原生依赖可用）。

## M2：真实数据优先

### M2a 发现与解析

实现有序路径上下文、逐角色解析、同名多目录、继承/默认回退、链接/别名链、循环与读取限制、非 UTF-8 路径诊断。

不要把缺 index.theme、缺部分常见角色、存在正常 symlink 等情况一律当作无效主题。区分元数据错误、可预览资产、可解析主题和可应用名字。

建立同名主题分散在两处的测试样例，确保角色可分别从两个来源解析。实际库语义若与猜测不同，以当前可验证行为和兼容性声明为准，留下 ADR，不能靠改测试隐藏偏差。

### M2b 解码与动画

从真实 Xcursor 字节生成统一预览模型。至少准备自己生成、可再分发的静态、多尺寸、动画、半透明、热点偏移样例，不下载未经许可的角色包进仓库。

选择一个解码实现并审查分配前边界。测试 TOC、offset、header、像素长度、乘法溢出、重复偏移、帧数和总预算；保留未知角色，不解析 CUR/ANI。

尺寸选择、delay 保护与颜色转换要有黄金期望值，不能仅“能解码不崩”就通过。

**完成条件：**无 GUI 的测试能验证真实像素/热点/顺序/时长；恶意或超限样例给出可定位的失败；未改变源文件和桌面。

## M3：能力探测与可注入设置后端

在 app 定义窄 settings 接口，在 platform 实现 GNOME/GIO 适配器。提供 fake backend；GIO 测试使用明确注入的 memory backend 和私有测试 schema。

先查 schema、key 类型/范围和可写性，再构造/使用设置对象。GNOME 之外或能力不明时仅提供浏览预览；有 GNOME schema 但不是 GNOME 会话不能视作可自动应用。

快照同时保存 effective value、user override 和本次修改项。错误保留 key、操作与底层原因。settings 通知路径不覆盖用户正在选择的候选主题。

正常测试不得构造连接用户默认 dconf 的写入 backend。仅靠 GSETTINGS_BACKEND 环境变量不够；测试必须能检查注入对象。

**完成条件：**无真实 GNOME 也能测试读/写/只读/缺 schema；不支持的环境仍可启动只读 UI。

## M4：只读 GTK 纵向切片

显示主题列表、来源诊断、当前配置、主题详情。打开或预览不得应用主题。

实现 real-data 静态卡片、选中主题的动画详情、热点标记、尺寸/放大区分。GTK 命名光标 API 不能充当选中主题资产加载器。

实现有界工作任务、取消/替换策略、request_id 丢弃迟到结果、按字节缓存、隐藏窗口暂停动画；避免把 spawn_local 里的阻塞扫描当成异步。

提供明确的 fixture 运行模式（参数名可选择），让 GUI 可只读使用仓库样例和内存 settings，完全不访问真实主题/桌面配置。后续自动 GUI 测试走此模式。

**完成条件：**能观察真实动画；快速切换 A/B 时不会串图；损坏主题不拖垮其他条目；fixture 模式无宿主副作用。

## M5：显式 Apply / Undo

实现架构规定的准备、重读、比较、差量写入、观察、部分失败和补偿逻辑。仅成功观测到本次有效变化时创建单级撤销记录；no-op 不改变记录。

测试 A→B→C→Undo=B；外部把 C 改成 D 后 Undo 不自动覆盖；原先未设 user value 时恢复 reset 语义；只改主题不改大小；补偿失败可见；超时不误报“什么都没变”。

不要调用 GSettings.revert 来假装撤销已写入设置。不要在 Drop 或重启时自动恢复。Apply 执行中禁止第二次同类写操作；已开始写入的操作不能以简单取消按钮许诺无副作用。

真实 GNOME 验收必须由用户明确发起，不能让测试脚本顺手改开发者桌面。

**完成条件：**fake/内存后端的正反向路径都有证据；GUI 成功文案不声称已视觉验证。

## M6：验证、诊断与交付

提供只读、默认脱敏的诊断文本；补充键盘操作、缩放、明暗样式和出错状态。不得自动收集或上传诊断。

核心命令（工作区与锁文件建立后执行）：

```bash
cargo fmt --all --check
cargo test --locked -p cursormochi-core -p cursormochi-app
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --release -p cursormochi-gtk --locked
```

不机械加入 --all-features。若有多个受支持 feature 组合，在 CI 明确列出，并分别说明 runtime 要求。缺少依赖导致无法执行的命令要原样报告，不跳过后写“全部通过”。

可增加性质测试、短时 fuzz 或依赖公告/许可证检查；额外工具若未安装或公告库不可用，应记录阻碍，不得把“未检查”写成“没有问题”。

交付 README 的安装/构建说明、架构简图、已知限制、首个版本的支持状态与 `docs/VALIDATION.md`。不必制作二进制发行包，尤其不默认引入 Flatpak/AppImage。

## 阶段推进方式

每阶段先检查再继续，不要求用户逐项批准设计细节。以下是真正需要停下对应写入动作的情况：不可恢复的用户数据风险、所需系统权限未授权、无法确定依赖/素材分发权限。其他独立安全工作继续，不因此只交空计划。

如果格式解析或 GUI 条件不足，保留已完成闭环，清楚标记剩余工作，不通过缩减测试、模拟成功或扩大范围来回避阻碍。

## 最终报告格式

- 已实现：具体行为，不只列文件名。
- 实际结构：与任务书的差异及理由。
- 自动检查：命令、环境、退出结果、关键输出。
- 隔离 GUI：显示后端、fixture 模式、观察结果。
- 真实桌面：谁发起、在哪个会话、验证到哪一步；没做则写未验证。
- 未解决项：具体失败、风险、复现方式。
- 支持声明：目标与已验证平台分开。

完成后停止于 v0.1，不自行进入 Windows 导入或商店阶段。
