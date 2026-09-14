# 验证记录 — 2026-09-14

交付状态：**v0.1 候选版本，宿主应用／撤销待用户发起人工验收**。这份文件记录已经执行的行为，不把任务矩阵本身当作 PASS。

## 实际环境

Ubuntu 26.04.1 LTS，x86_64，GNOME Shell 50.1，Wayland；Rust/Cargo 1.98.0；GTK 4.22.4；GLib/GIO 2.88.0；libXcursor 1.2.3。GUI 实际后端 `GdkWaylandDisplay`，`window.scale_factor()` 为 **2**。

## 自动检查

最终执行 `./scripts/check.sh`，退出 **0**，日志 `target/qa/check.log`。该脚本依次执行：

| 命令 | 状态 | 结果 |
|---|---|---|
| `glib-compile-schemas tests/schemas` | PASS | 私有测试 schema，未安装到宿主 schema 目录 |
| `cargo fmt --all --check` | PASS | 无格式差异 |
| `cargo test --locked -p cursormochi-core -p cursormochi-app` | PASS | core 10、app 9 项 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS | 无警告 |
| `cargo test --workspace --locked` | PASS | 共32项：core 10、app 9、platform 12、GTK worker 1；0失败 |
| `cargo build --release -p cursormochi-gtk --locked` | PASS | `target/release/cursor-mochi` |
| `cargo tree --locked -p cursormochi-app` | PASS | app → core，core无外部依赖 |
| `python3 scripts/reference-xcursor.py` | PASS | libXcursor 对静态、多帧、尺寸、像素、热点、延迟、继承的黄金值验证 |
| `target/debug/cursor-mochi --diagnose` | PASS | 本机发现35个主题，扫描诊断0；Adwaita、DMZ、Yaru等实际资产成功解码；来源已脱敏 |

测试使用临时文件、Fake 和 `gio::memory_settings_backend_new()` 明确注入对象。没有自动测试写入用户 dconf。私有 schema 包含范围约束及故意缺失的键；不靠 `GSETTINGS_BACKEND=memory` 作为唯一隔离措施。

构建中曾修复普通编译／Clippy问题，最后上述全套命令重新运行通过。参考比对最初暴露继承空白分隔和 libXcursor 路径初始化缓存问题，修正后通过；没有删除相应比对。

## GUI 实测

执行的可复现形式：

```sh
mkdir -p target/qa
timeout 25s dbus-run-session -- env GTK_A11Y=none GDK_BACKEND=wayland \
  GTK_THEME=Adwaita target/release/cursor-mochi --smoke-test
```

另分别使用 `GTK_THEME=Adwaita:dark`、`GTK_THEME=HighContrast` 执行，三次均退出0。测试启动项目 fixture、生成真实动画纹理、查看多帧、把检查放大从1×改为3×、设置搜索框焦点、快速替换角色、确认Apply/Undo禁用、保存窗口内容快照后退出。窗口内容快照已打开检查，颜色、热点、标签可读。

| 样式 | 实际后端 | 实际缩放 | 状态 | 内容快照 |
|---|---|---|---|---|
| Adwaita 浅色 | Wayland | 2 | PASS | [浅色](screenshots/preview-light.png) |
| Adwaita 深色 | Wayland | 2 | PASS | [深色](screenshots/preview-dark.png) |
| HighContrast | Wayland | 2 | PASS | [高对比](screenshots/preview-highcontrast.png) |

日志为 `target/qa/gui-wayland.log`、`gui-wayland-dark-2x.log`、`gui-wayland-highcontrast.log`，均包含动画和替换选择 PASS。

注意：试过 `GDK_SCALE=1`，但 Wayland 窗口报告仍为2。**不能据此记100%通过。** PNG按逻辑内容尺寸导出，不是物理屏幕缩放截图。100%、分数缩放、混合DPI：NOT RUN。

原用户D-Bus会话的 portal 查询出现超时，首轮进程由实施者终止，未记通过；后续使用独立D-Bus会话完成只读GUI检查。独立D-Bus下 portal服务退出有警告，应用自身烟雾检查退出0，无GTK应用崩溃。这不是对普通用户会话启动时间的性能结论。

## 对照验收矩阵

| 验收项 | 状态 | 已有证据／边界 |
|---|---|---|
| A01–A03 | PASS | 无GUI core/app测试、依赖树、锁定构建 |
| A04 | NOT RUN | 声明的低版本native基线未实测；本机版本构建已通过 |
| A05 | NOT RUN | 未模拟缺工具；无Xvfb，因此未执行Xvfb分支 |
| B01–B09 | PASS | 临时空目录、XDG规则、覆盖路径、同名逐角色、无index、继承、环、别名／坏链 |
| B10 | NOT RUN | 实现明确alternatives中转准入；未单独建立该系统布局回归样例 |
| B11 | PASS | 非UTF-8条目诊断、不用lossy作为身份；同展示名专门用例未执行 |
| B12 | PASS | FIFO非阻塞拒绝、普通输入超限；设备/socket/持续增长竞态专门用例未执行 |
| B13 | PASS | 预览后删除文件重读失败；缓存按完整输入重新核对，刷新重建根上下文 |
| B14 | PASS | 发现／有效路径分离，UI及诊断始终提示宿主路径待验证 |
| C01–C08 | PASS | 黄金样例、非方形、半透明、动画时序、截断、偏移、重复TOC、输出预算 |
| C09 | PASS | fixture B资产加载，内存后端只读；没有自动进行真实桌面A/B修改试验 |
| C10 | PASS | worker A→坏B→C最新请求测试，GUI角色快速替换 |
| C11 | PASS | 按字节LRU淘汰及内容替换测试；长期RSS压力测量未执行 |
| C12 | NOT RUN | 隐藏／最小化暂停已实现；尚未做专门GUI行为断言 |
| C13 | PASS | 扫描／解码取消、队列最新请求测试；不宣称硬终止阻塞线程 |
| C14–C15 | PASS | 继承／回退来源、错误局部隔离及可再次选择 |
| D01–D04 | PASS | 私有schema缺失/类型/范围、只读注入、纯读取无写入；非GNOME检测有实现，未真实启动其他桌面 |
| D05–D10 | PASS | 仅主题差量、no-op、单级Undo、reset、计划及撤销外部冲突 |
| D11–D14 | PASS | setter错误、部分失败、补偿失败、补偿前外部修改、延迟观察和读失败／不确定 |
| D15 | PASS | GIO通知测试；UI当前区独立刷新，候选选择不绑定设置 |
| D16–D17 | PASS | Controller忙碌串行化；新Controller无历史，无自动恢复路径 |
| D18 | NOT RUN | 窗口等待写入完成的处理已实现；fixture写入禁用，未专门注入GUI中途关窗 |
| E01 | PASS | 原创fixture + 显式内存backend + 原生Wayland只读窗口 |
| E02 | PASS | 原生控件及程序化焦点检查；完整真人键盘/读屏验收未执行 |
| E03 | PASS | 浅色、深色、高对比真实渲染内容快照 |
| E04 | PASS | 实际2×下1×/3×检查放大；100%、分数缩放、混合DPI NOT RUN |
| E05–E07 | NOT RUN | GNOME宿主Apply/Undo、GTK/Qt/浏览器真实鼠标及GPU专项未验收 |
| E08 | PASS | fixture只读模式正常浏览；其他桌面原生会话NOT RUN |
| E09 | PASS | 脱敏函数测试、实际诊断输出与源码仅剪贴板写入；未自动上传 |

PASS表示该行列出的自动／只读证据已通过；行内明确列出的更广边界仍未验证，不代表全平台支持。

## 用户发起的真实桌面验收（未执行）

任务书 `CODEX_START.txt` 第4项及 `docs/IMPLEMENTATION_V2.md` M5明确要求真实GNOME应用/撤销由用户发起。因此实施过程中没有替用户点击真实Apply/Undo。候选版本不能据此标成完整宿主兼容性验收通过。

用户验收时启动普通模式，记录原始有效值和user override；选择B后Apply，分别确认配置和实际鼠标；再选择C并Apply，Undo应回B。另在其他设置工具中改变配置后Undo应拒绝。逐一观察GTK、Qt和浏览器窗口。若最初无user override，Undo应恢复reset语义。此清单不是已执行记录。

## 剩余限制

- 真实宿主验收及上述NOT RUN项目，尤其100%/分数缩放、最小化、完整键盘/读屏、写入中关窗和长期内存压力测试。
- 默认本机libXcursor路径策略不保证所有宿主loader相同；出现坏文件时保留错误，可能比参考loader继续找候选更保守。
- 全部同UID文件系统竞态、网络文件系统硬超时、公开不可信包的进程隔离不在v0.1承诺内。
- 未进行独立安全审计或依赖公告库扫描；不声明零漏洞。

未推送、发布、安装系统组件、导入／删除用户主题或修改远程配置。

## 后续修复：列表点击与英文界面

根据用户反馈，公共Label不再默认允许选中文字；主题行Label同时不作为指针目标，使GTK ListBoxRow接收点击。来源路径仍可复制。应用按钮、状态、提示和设置错误文案统一为英文，用户安装的主题名称保持原样。

`./scripts/check.sh` 再次退出0（32项测试，fmt、Clippy、release通过），日志 `target/qa/check-english.log`。只读Wayland smoke检查退出0，并确认GTK实际pointer pick目标为ListBoxRow；日志 `target/qa/gui-english.log`。该次窗口报告scale=1，英文界面快照已检查：`target/qa/english-preview.png`。未修改宿主设置。
