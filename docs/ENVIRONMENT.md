# 环境与依赖核对（2026-09-14）

目标：Ubuntu 26.04 / GNOME 50 / Wayland / x86_64。当前实际：Ubuntu 26.04.1 LTS；Rust/Cargo 1.98.0；GTK 4.22.4；GLib/GIO 2.88.0；libXcursor 1.2.3；环境报告 `ubuntu:GNOME`、`wayland`。GUI 实际返回 GdkWaylandDisplay。另以 `gnome-shell --version` 确认为 GNOME Shell 50.1。

开始时没有 Git 仓库，只有架构文档；随后用户导入任务 ZIP。已保留 ZIP、根架构文档，并导入 AGENTS、CODEX_START 和 docs 任务文件。没有覆盖已有实现，没有建立远程仓库或推送。

## 选择

- 第一方 Rust 下限 1.98，实际以本机 1.98.0 验证。不承诺未运行的旧工具链。
- gtk4 0.9.7 / gio、glib 0.20.12：本机已有的同代兼容组合；未开启高版本 GTK feature。gtk-rs 为持续维护项目，此选定分支不是最新主线；MIT。直接 native 需求 GTK ≥4.0、GLib ≥2.66，旧运行库未实际测试。
- rustix 1.1.4：安全 API 的非阻塞、NOFOLLOW 文件打开，维护中的 Rust 系统接口库；Apache-2.0 / MIT / Apache-2.0 WITH LLVM-exception。上游声明 MSRV 1.63。第一方无 unsafe，不等于依赖或 native 无 unsafe。
- core 无外部库，app 只依赖 core；GIO 不进入这两层。Cargo.lock 固定实际 78 个外部 package 解析，具体数量以 `cargo metadata` 为准。
- 候选 xcursor 0.3.10 为 MIT；源代码保留了双份像素缓冲，并按输入 TOC 建立输出，没有本项目需要的分配前总帧／总像素预算。故未采用该运行时依赖，选择唯一的受控本地 Xcursor image 解码器。评估来源：<https://docs.rs/xcursor/0.3.10/src/xcursor/parser.rs.html>。
- 未进行完整依赖安全审计或公告库审查，不声明“无漏洞”。许可证元数据记录与 Cargo.lock 不替代传递依赖审计。

## 路径依据

开发期通过 libXcursor 1.2.3 的 `XcursorLibraryPath` 只读查询得到 `~/.icons:/usr/share/icons:/usr/share/pixmaps`。应用不链接 Xlib，不动态加载 libXcursor。有效 XCURSOR_PATH 覆盖这套顺序；XDG 目录用于发现并集，不能据此宣称宿主采用 XDG 路径。

`reference-xcursor.py` 在隔离子进程中设置 XCURSOR_PATH，加载原创新样例，对帧数、nominal、实际宽高、热点、ARGB、延迟和继承做黄金值比对。初次比对发现参考加载器缓存路径，以及 Inherits 空白分隔行为；修正测试初始化顺序与继承解析后通过。没有改动宿主设置。
