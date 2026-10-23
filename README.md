# Game Account Switcher (Riot Account Manager v4)

App Windows nhỏ gọn để đổi tài khoản **Riot Client** (League of Legends, VALORANT) và **Garena – Delta Force** chỉ với một cú bấm. Đây là bản viết lại (v4) của Riot Account Manager v3: giữ nguyên luồng hoạt động, viết mới toàn bộ code bằng Tauri 2 + React + Rust.

## Chuyển từ bản cũ (v3 / bản Java)

Lần đầu mở v4, app tự đọc dữ liệu ở `%LOCALAPPDATA%\RiotAccountManager\`:

- `accounts.dat`, cả định dạng v3 (`RAM2:` + DPAPI) lẫn định dạng Java cũ (AES theo MachineGuid). Các tài khoản được thêm vào danh sách Riot, server (region) được ghi vào phần ghi chú.
- `config.json`: đường dẫn Riot Client, *Khởi động cùng Windows*, *Tự bấm đăng nhập*.

File cũ chỉ được **đọc**, không bị sửa hay xoá, nên bản v3 vẫn chạy bình thường nếu cần quay lại. Việc chuyển chỉ chạy một lần; kết quả hiện thành thông báo khi mở app và ghi trong `switch.log`.

## Tính năng

- Lưu nhiều tài khoản Riot và Garena; mật khẩu mã hoá bằng **Windows DPAPI**, không gửi đi đâu.
- **Tự đăng xuất** tài khoản cũ (đóng launcher + xoá phiên "Ghi nhớ đăng nhập") rồi đăng nhập tài khoản mới.
- **Riot:** hỏi chơi **LoL** hay **VALORANT** (hoặc chỉ đăng nhập), đăng nhập xong game tự mở. Có thể đặt game mặc định cho từng tài khoản.
- **Delta Force (Garena):** đăng nhập launcher rồi tự bấm nút *Khởi động*.
- **Khay hệ thống:** menu liệt kê mọi tài khoản để đổi nhanh; đóng cửa sổ thì thu xuống khay.
- **Phím tắt toàn cục:** `Ctrl+Alt+A` hiện/ẩn cửa sổ, `Ctrl+Alt+1…9` đăng nhập tài khoản số 1…9 (Riot trước, rồi Garena).
- Xuất / nhập sao lưu mã hoá AES-256 bằng mật khẩu, khởi động cùng Windows, giao diện sáng/tối theo hệ thống.

## Cách hoạt động (và giới hạn)

App **chỉ mô phỏng bàn phím và chuột** như người dùng thật: không hook, không inject DLL, không đọc bộ nhớ tiến trình khác, không gọi API đăng nhập của Riot/Garena.

| Bước | Riot Client | Garena / Delta Force |
|---|---|---|
| Đăng xuất | Đóng Riot Client + game, xoá `%LOCALAPPDATA%\Riot Games\Riot Client\Data\RiotGamesPrivateSettings.yaml` | Đóng mọi tiến trình trong thư mục launcher (+ tiến trình game bạn khai báo), xoá các file phiên bạn khai báo |
| Mở launcher | `RiotClientServices.exe --launch-product=… --launch-patchline=live` | Chạy file launcher bạn chọn |
| Điền tài khoản | Vị trí đã hiệu chỉnh → UI Automation → dò ô nhập ở cột trái | Vị trí đã hiệu chỉnh → UI Automation → dò ô nhập giữa cửa sổ |
| Mở game | Riot Client tự mở game sau khi đăng nhập (app nhắc lại lệnh nếu chưa thấy game) | Bấm vào vị trí nút *Khởi động* đã hiệu chỉnh |

Garena không công bố cấu trúc launcher, nên phần Delta Force cần **hiệu chỉnh một lần** trong *Cài đặt → Garena*: mở launcher tới đúng màn hình, bấm **Ghi vị trí**, rồi trong 5 giây đặt chuột lên ô *Tài khoản* (và nút *Khởi động* ở màn hình sau khi đăng nhập). Chỉ hỗ trợ đăng nhập bằng **tài khoản Garena** (không phải Facebook/Google/QR).

Trong lúc app đang điền, đừng chạm chuột/bàn phím. Nếu điền thất bại, xem `%APPDATA%\GameAccountSwitcher\switch.log`.

## Build

Cần Windows 10/11, [Node.js](https://nodejs.org) ≥ 18 và [Rust](https://rustup.rs) (stable).

```bash
npm install
npm run tauri:dev      # chạy thử
npm run tauri:build    # tạo installer: target/release/bundle/nsis/*-setup.exe
cargo test -p gas-core # test phần lõi
```

`npm run dev` mở giao diện trong trình duyệt với dữ liệu giả (không đụng tới launcher thật) để chỉnh UI nhanh.

Không muốn cài công cụ? Đẩy repo lên GitHub: workflow `.github/workflows/build.yml` tự build trên Windows và đính kèm file `*-setup.exe` trong tab **Actions** (đẩy tag `v*` thì tạo luôn Release).

## Cấu trúc

```
crates/core/        Lõi Rust: model, mã hoá, lưu trữ, Riot, Garena, quy trình đổi tài khoản
  src/win/          Win32: tiến trình, cửa sổ, bàn phím/chuột, UI Automation, clipboard
src-tauri/          Vỏ Tauri: lệnh cho UI, khay hệ thống, phím tắt
src/                Giao diện React
```

Dữ liệu nằm ở `%APPDATA%\GameAccountSwitcher\`: `accounts.dat` (DPAPI), `accounts.bak`, `settings.json`, `state.json`, `switch.log`.

## Lưu ý

Dùng cho các tài khoản của chính bạn. Hãy tuân thủ điều khoản dịch vụ của Riot Games và Garena.
