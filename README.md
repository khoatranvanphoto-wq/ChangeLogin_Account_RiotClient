# Riot Account Manager

[![GitHub](https://img.shields.io/badge/GitHub-Repository-blue?style=flat-square&logo=github)](https://github.com/DangNghia17/ChangeLogin_Account_RiotClient)
[![Release](https://img.shields.io/badge/Release-v3.2.0-green?style=flat-square)](https://github.com/DangNghia17/ChangeLogin_Account_RiotClient/releases)
[![License](https://img.shields.io/badge/License-MIT-yellow?style=flat-square)](LICENSE)
[![Tauri](https://img.shields.io/badge/Tauri-2.x-24C8DB?style=flat-square&logo=tauri)](https://tauri.app/)
[![React](https://img.shields.io/badge/React-18-61DAFB?style=flat-square&logo=react)](https://react.dev/)

> Ứng dụng desktop nhẹ giúp quản lý và đăng nhập nhanh nhiều tài khoản Riot Games (League of Legends, VALORANT, …).

---

## Giới thiệu

**Riot Account Manager** là công cụ desktop (Tauri + React) giúp bạn lưu trữ nhiều tài khoản Riot Games một cách an toàn và **tự động điền thông tin đăng nhập** vào Riot Client chỉ với một cú nhấp chuột. Thay vì gõ lại username/password mỗi lần đổi tài khoản, bạn chỉ cần chọn tài khoản và bấm **Đăng nhập**.

Ứng dụng hoạt động hoàn toàn cục bộ, mã hóa dữ liệu nhạy cảm và **không gửi bất kỳ thông tin nào lên server**.

## Tính năng nổi bật

- 🗂️ **Quản lý tài khoản** — lưu trữ nhiều tài khoản Riot trong một nơi.
- ➕ ✏️ 🗑️ **Thêm / Sửa / Xóa** — cụm nút thao tác trực quan, icon-only kèm tooltip.
- ⚠️ **Xác nhận trước khi xóa** — hộp thoại xác nhận để tránh xóa nhầm, không thể hoàn tác.
- 👋 **Hướng dẫn người dùng lần đầu** — màn hình chào mừng (first-run) thân thiện.
- 🔐 **Quản lý trạng thái đăng nhập** — nhận biết tài khoản đang đăng nhập, không đăng nhập lại thừa.
- 🚪 **Đăng xuất** — thoát phiên hiện tại để chuyển sang tài khoản khác.
- 🎨 **Giao diện cải tiến** — nút bo góc hiện đại, hiệu ứng hover/active, shadow nhẹ.
- ✨ **Trải nghiệm người dùng tốt hơn** — toast thông báo, badge trạng thái, animation mượt.
- 📤 **Export dữ liệu** — xuất toàn bộ dữ liệu ra file `.backup`.
- 📥 **Import dữ liệu** — khôi phục dữ liệu từ file backup.
- 🔑 **Sao lưu được mã hóa bằng mật khẩu** — AES-256-GCM + PBKDF2 (Web Crypto API); mật khẩu không bao giờ được lưu.
- ♻️ **Khôi phục dữ liệu từ file backup** — có xác nhận trước khi ghi đè và kiểm tra toàn vẹn.
- 🌗 **Light / Dark Theme** — hỗ trợ giao diện Sáng, Tối và theo Hệ thống.
- 💾 **Ghi nhớ lựa chọn giao diện** — giữ nguyên theme sau khi khởi động lại.
- 🌐 **Đa ngôn ngữ** — Tiếng Việt & Tiếng Anh.
- 🛡️ **An toàn với game** — chỉ mô phỏng thao tác bàn phím/chuột, KHÔNG hook/inject, tương thích Vanguard.

## Công nghệ sử dụng

| Thành phần | Công nghệ |
|------------|-----------|
| Khung ứng dụng desktop | [Tauri 2.x](https://tauri.app/) |
| Giao diện | [React 18](https://react.dev/) + TypeScript |
| Build tool | [Vite 5](https://vitejs.dev/) |
| Lõi nghiệp vụ | Rust (crate `ram-core`) |
| Mã hóa dữ liệu | AES (crate `aes`/`ecb`) + SHA-256 |
| Đóng gói | NSIS installer (Windows) |

## Cài đặt

### Dành cho người dùng cuối

1. Tải `*-setup.exe` hoặc `RiotAccountManager-portable.zip` từ trang [Releases](https://github.com/DangNghia17/ChangeLogin_Account_RiotClient/releases).
2. Nếu Windows SmartScreen cảnh báo: bấm **More info → Run anyway** (app chưa ký code signing).
3. Chạy installer và mở app từ Start Menu.

> **Yêu cầu:** Windows 10/11, Riot Client đã cài, WebView2 (có sẵn trên Windows 10/11). Không cần Java.

### Dành cho nhà phát triển

```bash
# Yêu cầu: Node.js >= 18, Rust toolchain (rustup)
git clone https://github.com/DangNghia17/ChangeLogin_Account_RiotClient.git
cd ChangeLogin_Account_RiotClient
npm install            # cài deps ở root + app-tauri (qua postinstall)
```

## Chạy dự án

```bash
# Xem trước UI trên trình duyệt (web preview, dùng mock store)
npm run web

# Chạy app desktop đầy đủ (Tauri dev)
npm run tauri:dev
```

## Build

```bash
# Build phần web (TypeScript + Vite)
npm run build

# Build app desktop (installer)
npm run tauri:build

# Kiểm tra chất lượng
npm run check     # typecheck web + cargo check/test cho ram-core
npm run lint      # tsc --noEmit
npm run test      # cargo test -p ram-core
```

## Cấu trúc project

```
.
├── app-tauri/                 # Ứng dụng Tauri + React
│   ├── src/                   # Mã nguồn frontend (React/TypeScript)
│   │   ├── components/        # AccountTable, ConfigPanel, ConfirmDialog, Toast
│   │   ├── dialogs/           # Account, Settings, About, Welcome, LoginStatus, Backup
│   │   ├── i18n/              # Đa ngôn ngữ (vi/en)
│   │   ├── theme/             # ThemeContext (Light/Dark/System)
│   │   ├── lib/               # api, platform, mock-store, backup (mã hóa)
│   │   ├── App.tsx            # Thành phần gốc
│   │   └── styles.css         # Toàn bộ style (biến theme sáng/tối)
│   ├── src-tauri/             # Lớp shell Tauri (Rust) + cấu hình
│   └── crates/core/           # ram-core: lõi nghiệp vụ Rust (store, crypto, riot…)
├── docs/                      # Tài liệu bổ sung
├── scripts/                   # Script tiện ích (doctor.mjs…)
├── README.md                  # Tài liệu này
├── RELEASE_v3.0.0.md          # Ghi chú phát hành
└── REPORT_v3.0.0.md           # Báo cáo thay đổi v3.0.0
```

## Hướng dẫn sử dụng nhanh

1. **Cấu hình Riot Client** — bấm **Chọn** để trỏ tới `RiotClientServices.exe` (thường ở `C:\Riot Games\Riot Client\RiotClientServices.exe`).
2. **Thêm tài khoản** — bấm nút ➕, nhập Username/Password/Region, bấm **Lưu**.
3. **Đăng nhập** — chọn tài khoản → bấm **Đăng nhập** để tự động điền vào Riot Client.
4. **Sửa / Xóa** — chọn tài khoản rồi bấm ✏️ hoặc 🗑️ (xóa sẽ hỏi xác nhận).
5. **Đăng xuất** — nếu đang ở phiên một tài khoản, bấm Đăng nhập lại tài khoản đó để mở hộp thoại trạng thái và **Đăng xuất**.
6. **Sao lưu / Khôi phục** — mở **Cài đặt → Sao lưu & Khôi phục**, chọn **Xuất dữ liệu** (đặt mật khẩu để mã hóa file `.backup`) hoặc **Nhập dữ liệu** (chọn file, nhập mật khẩu, xác nhận ghi đè).
7. **Đổi giao diện** — bấm nút ☀/🌙 trên thanh tiêu đề để chuyển nhanh Sáng/Tối, hoặc chọn Sáng/Tối/Hệ thống trong **Cài đặt → Giao diện**.

## An toàn & Bảo mật

- **Không ảnh hưởng game:** chỉ dùng API chuẩn của hệ điều hành để mô phỏng thao tác bàn phím/chuột. KHÔNG hook process, KHÔNG inject DLL, tương thích Vanguard.
- **Không lộ thông tin:** dữ liệu nhạy cảm được mã hóa và lưu cục bộ trong `%LOCALAPPDATA%\RiotAccountManager\`. KHÔNG kết nối internet, KHÔNG gửi dữ liệu đi.
- **Mã nguồn mở:** bạn có thể tự kiểm tra toàn bộ mã nguồn.

## Roadmap

- [x] Import/Export & sao lưu dữ liệu có mã hóa. ✅ (v3.1.0)
- [x] Tùy biến theme (sáng/tối) + ghi nhớ lựa chọn. ✅ (v3.1.0)
- [ ] Tìm kiếm / lọc tài khoản trong danh sách.
- [ ] Sắp xếp & nhóm tài khoản theo region/ghi chú.
- [ ] Phím tắt toàn cục để đăng nhập nhanh.
- [ ] Hỗ trợ thêm nền tảng (macOS/Linux) nếu khả thi.

## License

Phát hành theo giấy phép **MIT** — xem file [LICENSE](LICENSE) để biết chi tiết.

## Changelog

### v3.2.0

- Auto-login không còn click theo tỉ lệ cửa sổ / pixel cố định (lệch DPI, size, bản Riot).
- Tìm ô username/password qua Windows UI Automation (cây accessibility thật của Riot Client).
- Fallback bàn phím nếu form chưa lộ trong accessibility tree.
- Focus cửa sổ Riot bằng Win32 (bỏ PowerShell); không nhận nhầm cửa sổ app.

### v3.1.0

✨ **New Features**
- Import dữ liệu.
- Export dữ liệu.
- Sao lưu có mã hóa bằng mật khẩu (AES-256-GCM + PBKDF2).
- Khôi phục dữ liệu từ file backup.
- Hỗ trợ Light/Dark Theme (kèm tùy chọn theo Hệ thống).
- Ghi nhớ cài đặt giao diện giữa các lần mở ứng dụng.

🔒 **Security**
- Backup được mã hóa đầu-cuối bằng mật khẩu người dùng.
- Không lưu mật khẩu ở bất kỳ đâu.
- Kiểm tra tính toàn vẹn dữ liệu khi khôi phục (AES-GCM authentication + kiểm tra định dạng/phiên bản).

### v3.0.0
- Thiết kế lại cụm nút thao tác.
- Chuyển sang icon-only hiện đại.
- Bổ sung xác nhận trước khi xóa.
- Thêm hướng dẫn cho người dùng lần đầu.
- Cải thiện xử lý khi đã đăng nhập.
- Bổ sung chức năng đăng xuất.
- Refactor và tối ưu giao diện.
- Cải thiện trải nghiệm người dùng.

### v2.0.0
- Chuyển toàn bộ ứng dụng từ Java sang Tauri + React (native, không cần Java).
- Mã hóa và tự động nhập dữ liệu từ bản Java cũ.
- Hỗ trợ đa ngôn ngữ (vi/en), cài đặt khởi động cùng Windows.

---

**GitHub:** [https://github.com/DangNghia17/ChangeLogin_Account_RiotClient](https://github.com/DangNghia17/ChangeLogin_Account_RiotClient)
