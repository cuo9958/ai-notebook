## 编译与部署

### 一键构建

项目提供了针对不同操作系统的构建脚本，位于 `server/` 目录下：

- **Windows**: 双击运行 `build.bat`。
- **Linux / macOS**: 赋予执行权限后运行 `./build.sh` (`chmod +x build.sh && ./build.sh`)。

### 各环境编译指南

| 目标平台 | 建议命令 | 说明 |
|:---|:---|:---|
| **当前开发环境** | `cargo run` | 带调试信息，适合开发联调 |
| **生产发布 (Release)** | `cargo build --release` | 开启优化，体积更小 |
| **Linux 静态编译** | `cargo build --target x86_64-unknown-linux-musl` | 产物无外部依赖，适合 Docker 极简部署 |
| **Windows (Linux下交叉)** | `cargo build --target x86_64-pc-windows-gnu` | 需安装 `mingw-w64` 工具链 |
| **macOS (Linux下交叉)** | `cargo build --target x86_64-apple-darwin` | 需安装 `osxcross` 工具链 |

### Docker 容器化部署建议

本项目使用轻量级 SQLite 数据库和零依赖的发布模式，非常适合容器化。

```dockerfile
# 1. 构建阶段 (Builder)
FROM rust:slim AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

# 2. 运行阶段 (Runner)
FROM debian:bookworm-slim
WORKDIR /app
# 拷贝编译好的二进制和配置文件
COPY --from=builder /app/target/release/markdown-server /app/
COPY config.toml /app/

EXPOSE 3000
CMD ["./markdown-server"]
```