<p align="center">
  <img src="assets/logo.png" alt="ZUST Checker Logo" width="250">
</p>


<p align="center">
  浙江科技大学(ZUST)教务查询桌面应用。<br>
  查询成绩、GPA、课表、考试安排,支持选课与自动抢课。
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-GPL--3.0-blue.svg" alt="License"></a>
  <a href="https://github.com/EthanZer0/zust-checker/releases"><img src="https://img.shields.io/badge/platform-Windows%2011-blue" alt="Platform"></a>
  <a href="https://github.com/EthanZer0/zust-checker/releases"><img src="https://img.shields.io/badge/language-Rust%20%7C%20Svelte%205-orange" alt="Language"></a>
  <a href="https://github.com/EthanZer0/zust-checker/releases"><img src="https://img.shields.io/badge/version-0.1.0-green" alt="Version"></a>
</p>

---

## 特性

<div align="center">

| 成绩与 GPA | 课表 | 考试安排 |
|:---:|:---:|:---:|
| 按学期查看成绩<br>自动计算加权 GPA | 列表 / 表格双视图<br>按学期切换 | 查询考试时间与地点 |
| **选课与抢课** | **自动登录** | |
| 查看可选/已选课程<br>一键选课 + 定时抢课(自动重登) | CAS 单点登录<br>会话缓存,免重复输入 | |

</div>

---

## 技术栈

| 层 | 技术 |
|---|---|
| 桌面框架 | [Tauri v2](https://tauri.app) |
| 前端 | [Svelte 5](https://svelte.dev) + TypeScript + Vite |
| 后端 | Rust(纯核心库 `zust-core`) |
| HTTP | reqwest(rustls TLS) |

---

## 快速开始

```bash
# 安装前端依赖
npm install

# 开发模式(热更新)
npm run tauri dev

# 打包 Windows 安装程序
npm run tauri build
```

需要 Rust 工具链与 Node.js 18+。

---

## 项目结构

```
zust-checker/
├── src/               Svelte 5 前端
├── crates/zust-core/  核心 Rust 库(HTTP 会话、认证、API 客户端、抢课引擎)
└── src-tauri/         Tauri 桌面壳
```

---

## 免责声明

本项目仅供学习与个人使用,不保证功能持续可用。教务系统接口可能随时变更,使用本项目产生的任何后果由使用者自行承担。请勿用于任何违反学校规定或法律法规的用途。

---

## License

[GPL-3.0](LICENSE) © 2026 EthanZer0
