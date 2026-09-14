# CursorMochi

本地 Linux 光标主题浏览器，使用 Rust + GTK4/GIO。v0.1 **候选版本：宿主应用／撤销待人工验收**。

支持发现已有主题、查看真实 Xcursor 静态／动画、角色与标称尺寸选择、原始像素和检查放大、热点标记。GNOME 中只有显式点击“应用”或“撤销”才修改配置。主题切换默认不修改大小；撤销只保存本次运行中上一次成功修改，遇到外部变化会停止。

## 构建和运行

当前验证工具链：Rust 1.98.0。声明 Rust 下限 1.98；native API 基线 GTK 4.0、GLib/GIO 2.66（仅在本机 GTK 4.22.4 / GLib 2.88.0 实测，旧 runtime 未实测）。需要 `pkg-config`、GTK4 开发库及 C 链接器；Ubuntu 对应 `libgtk-4-dev`、`pkg-config`、`build-essential`。本项目不会自动安装系统软件。

```sh
cargo build --release -p cursormochi-gtk --locked
./target/release/cursor-mochi
```

使用应用时无需网络。首次 Cargo 构建可能下载锁定依赖。

只读样例模式使用项目自有资产和**明确注入的内存设置后端**：

```sh
glib-compile-schemas tests/schemas
cargo run --locked -p cursormochi-gtk -- --fixture
```

`--diagnose` 可在无显示环境输出只读、脱敏的本地发现与解析报告。

`--smoke-test` 隐含 `--fixture`，自动检查动画、快速替换选择和禁用的写入控件后退出。`--help` 显示参数。fixture 模式从源码目录加载样例；普通模式不依赖样例目录。

## 使用

1. 在列表选择主题，可搜索名称，用方向键切换。
2. 选择角色和文件内标称尺寸。放大只影响检查画布；1× 表示 GTK 逻辑像素，不保证等于屏幕物理像素。
3. 查看来源、继承／回退状态与原始动画延迟。坏角色不会阻止浏览其他主题。
4. 需要修改桌面时点击“应用”；仅勾选大小选项时才同时修改 GNOME 大小，最终范围由宿主 schema 校验。
5. “撤销上一次修改”只恢复该操作拥有的键，原先未设置用户覆盖时执行 reset。A→B→C 后撤销到 B；重启后无历史。

配置回读一致不代表所有窗口的实际鼠标都已更新。非 GNOME、缺 schema、沙箱或锁定键时降级只读。外部设置变化更新当前配置，不改变候选选择。进行写入时关闭窗口会等待有界回读结束，不会隐式撤销。

“复制诊断”只写本机剪贴板，默认脱敏 HOME 和个人搜索路径，无遥测或上传。

## 结构

```text
cursormochi-gtk ──→ app + platform + core
cursormochi-platform ──→ app + core
cursormochi-app ──→ core
cursormochi-core ──→ Rust std
```

后台只有一个有界工作线程，无服务和 Tokio。core 解析器在分配前检查输入、TOC、尺寸、帧数及像素总量。platform 通过安全 rustix API 非阻塞打开文件、检查文件句柄、限制读取范围；正常本地链接可读，但不宣称抵御同 UID 的全部文件系统竞态。

## 验证

```sh
./scripts/check.sh
python3 scripts/reference-xcursor.py
mkdir -p target/qa
dbus-run-session -- env GTK_A11Y=none GDK_BACKEND=wayland \
  target/debug/cursor-mochi --smoke-test
```

GUI 命令需要可用 Wayland 显示；`GTK_A11Y=none` 只用于隔离烟雾检查，普通运行保留 GTK 可访问性。本机会话 portal 服务存在启动超时，独立 D-Bus 会话的 fixture 检查可正常完成。不要把隔离 D-Bus 用于真实桌面 Apply 验收。

具体结果见 [VALIDATION](docs/VALIDATION.md)、[环境与依赖](docs/ENVIRONMENT.md) 和 [实现决策](docs/DECISIONS.md)。原始任务文件完整保留在 docs。

## 已知限制

- Linux x86_64 / Ubuntu 本机路径策略；GNOME 真实 Apply/Undo、Qt/浏览器、分数缩放仍须用户发起验收。
- 默认解析按本机 libXcursor 编译路径；发现额外 XDG 目录不代表 compositor 使用这些目录，界面始终标注路径待验证。
- 不支持 CUR/ANI、下载、主题安装／删除、商店、其他桌面写入或分发容器。仅预览已有本地 Xcursor。
- 动画延迟限制到 16–10000 ms 播放，保留原始值。错误状态使用稳定的英文错误码和中文操作建议。
- 当前详情提供一张静态第一帧卡片；列表不为所有主题同时建立图像纹理。

第一方代码与原创测试样例采用 MIT 许可；native 与传递依赖保留各自许可。
