# APPDATACleaner - 适用于 Windows 系统的 appdata 文件夹清理工具
完全开源免费的清理Appdata的小工具！完全使用 ChatGPT-4o 生成！

## 🖥系统要求
- Windows

## 使用方法

### 📦下载exe文件
- [发行版](https://github.com/TC999/AppDataCleaner/releases/latest)
- [CI 构建](https://github.com/TC999/AppDataCleaner/actions/workflows/ci.yml)

以上两种方法二选一，下载后直接解压运行即可。

### 运行
- 双击运行
- 点击“立即扫描”,软件会自动扫描Appdata文件夹，并显示扫描结果。
- 自行选择“删除”或“移动”（暂未实现）
> [!caution]
> 
> 请注意，删除操作不可逆，且没有反悔机会，请谨慎操作。

### 从源码编译
- 安装 Rust
- 克隆此仓库
```
git clone https://github.com/TC999/AppDataCleaner.git
```
- 进入项目目录
```
cd AppDataCleaner
```
- 调试
```
cargo run
```
- 编译
```
cargo build --release
```
- 编译产物在 target/release 目录下

## 📝许可证
本项目采用 [GPLv3 许可证](LICENSE)。