# 技术依据与来源

核对日期：2026-09-14。以下为上游规范/官方项目文档或库作者发布的 API 文档。

这些资料支撑 API 和格式语义，不证明 CursorMochi 已实现、已审计或在用户设备上通过测试。本任务包的架构、预算和验收门槛是项目建议。

## S01 — XDG 目录

freedesktop.org，XDG Base Directory Specification：
https://specifications.freedesktop.org/basedir/latest/

依据：XDG_DATA_HOME/DIRS 等默认值、绝对路径规则与不同用途的目录划分。XDG 规范本身不能证明每个 cursor loader 都按相同路径顺序查找。

## S02 — Xcursor 格式、主题与路径

X.Org 官方 Xcursor 手册（归档版本）：
https://xorg.freedesktop.org/archive/X11R7.5/doc/man/man3/Xcursor.3.html

依据：标称尺寸、动画帧、热点/延迟、主题/逐文件查找/继承和 XCURSOR_PATH。该手册是历史版本，**不作为当前发行版默认搜索路径的唯一依据**；实施时核对所用库与主机行为。格式细节存在歧义时以当前实现、样例与明确策略记录为准。

## S03 — GDK 命名光标查找

GTK/GDK：Cursor.new_from_name：
https://docs.gtk.org/gdk4/ctor.Cursor.new_from_name.html

依据：按名称查找当前 cursor theme，而非任意指定主题目录。

## S04 — 像素纹理与格式

GTK/GDK：MemoryTexture.new：
https://docs.gtk.org/gdk4/ctor.MemoryTexture.new.html

GTK/GDK：MemoryFormat：
https://docs.gtk.org/gdk4/enum.MemoryFormat.html

依据：纹理可从内存像素构建，需正确描述尺寸、stride、字节/通道和 alpha 格式。API 页面展示的文档库版本不等于用户已安装版本。

## S05 — GTK 主循环与阻塞工作

gtk-rs 官方教程：The Main Event Loop：
https://gtk-rs.org/gtk4-rs/stable/latest/book/main_event_loop.html

依据：长阻塞操作影响主循环、工作线程与主循环消息协作、Gtk 对象线程约束。任务取消、资源预算和队列设计属于本项目进一步要求。

## S06 — Rust Xcursor 解析候选

xcursor crate 作者的 API 文档：
https://docs.rs/xcursor/latest/xcursor/
https://docs.rs/xcursor/latest/xcursor/parser/index.html

依据：存在主题/格式解析能力可供评估。不据此保证安全、维护质量或可直接满足资源限额；不在任务书中锁死某个未经测试的版本。

## S07 — GIO 能力探测与构造

https://docs.gtk.org/gio/method.SettingsSchemaSource.lookup.html
https://docs.gtk.org/gio/method.Settings.is_writable.html
https://docs.gtk.org/gio/ctor.Settings.new_full.html

依据：schema 查找、可写性查询以及可指定 backend 的 settings 构造。

## S08 — 有效值与用户覆盖

https://docs.gtk.org/gio/method.Settings.get_user_value.html

依据：用户覆盖与有效值可能不同；reset 与没有用户覆盖的状态有关。

## S09 — 写入与同步

https://docs.gtk.org/gio/type_func.Settings.sync.html
https://docs.gtk.org/gio/method.Settings.apply.html

依据：写入是异步的，sync 是阻塞操作；apply 不是应用自有的持久撤销历史或全局竞争锁。

## S10 — revert 的范围

https://docs.gtk.org/gio/method.Settings.revert.html

依据：revert 撤回的是 delay-apply 模式下尚未应用的改变。

## S11 — 测试用内存 settings

https://docs.gtk.org/gio/func.memory_settings_backend_new.html

依据：memory backend 不写持久后端。测试是否真正注入该 backend 仍需代码与用例保证。

## S12 — gtk-rs 的 native 版本与 feature

https://gtk-rs.org/gtk4-rs/stable/latest/book/project_setup.html

依据：先核对 GTK native 版本，再决定使用的 API feature。教程示例里的版本不应被当成当前最新版本，也不能代替最低支持基线的设计。

## S13 — Cargo 锁文件

https://doc.rust-lang.org/cargo/guide/cargo-toml-vs-cargo-lock.html

依据：Cargo.toml 的版本约束与 Cargo.lock 的具体依赖解析作用。Rust 依赖锁文件不负责锁定宿主 GTK/GLib 等发行版包。

## S14 — 给 Codex 的可复用项目指导

OpenAI 官方 Codex 文档：
https://developers.openai.com/codex/agent-configuration/agents-md
https://developers.openai.com/codex/learn/best-practices

依据：AGENTS.md 适合持续项目约束；任务目标、上下文、限制和完成条件应清楚。不要把自然语言指导当成实际权限控制，本包并未配置自动审查或自动化任务。
