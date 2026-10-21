# Riot Account Manager

[![GitHub](https://img.shields.io/badge/GitHub-Repository-blue?style=flat-square&logo=github)](https://github.com/DangNghia17/ChangeLogin_Account_RiotClient)
[![Release](https://img.shields.io/badge/Release-v3.2.0-green?style=flat-square)](https://github.com/DangNghia17/ChangeLogin_Account_RiotClient/releases)
[![License](https://img.shields.io/badge/License-MIT-yellow?style=flat-square)](LICENSE)

## Introduction

Riot Account Manager is a tool to help manage and quickly login to multiple Riot Games accounts (League of Legends, VALORANT, etc.). The application helps you switch between accounts quickly and conveniently.

## Features

- ✅ Manage multiple Riot Games accounts in one place
- ✅ Auto login - just select account and click button
- ✅ Open and focus on Riot Client from the application
- ✅ Display Riot Client status (running/not running)
- ✅ Securely store login information (AES-256 encryption)
- ✅ Quickly switch accounts, save time
- ✅ Multi-language support (Vietnamese and English)

## Benefits

Compared to manual login:
- ⏱️ **Save time**: From 30-60 seconds → 2-3 seconds
- 🎯 **Accurate**: No worries about wrong username/password
- 🎨 **Convenient**: Easy to manage multiple accounts
- 🔒 **Safe**: Encrypted and stored locally
- ⚡ **Efficient**: Automate the login process

## Safety and Security

### Does it affect the game?

**NO.** The application is completely safe:
- Only uses standard Windows API (Robot, Process)
- NO process hooking, NO DLL injection, NO memory interference
- Fully compatible with Vanguard (Riot's anti-cheat)
- Only simulates keyboard/mouse actions like a real user
- Does not interfere with game client or game logic

### Will information be leaked?

**NO.** Information is maximally protected:
- AES-256 encryption for all sensitive data
- Stored locally in `%LOCALAPPDATA%\RiotAccountManager\`
- NO data sent to server, NO internet connection
- Open source (MIT License) - you can check
- Only you have access to data on your computer

## 📦 Download

### Portable Version (Recommended)
- **File:** `RiotAccountManager-portable.zip` or `RiotAccountManager.exe`
- **Contents:** Native Tauri app, lightweight, ready to use
- **Target:** End users

### Installer
- **NSIS `.exe`** or **WiX `.msi`** — install with shortcuts

### Source code
- Clone the repo from GitHub — see `README_DEV.md` to build

📥 **Download:** [Releases](https://github.com/DangNghia17/ChangeLogin_Account_RiotClient/releases)

## Installation

1. Download **Portable Version** from [Releases](https://github.com/DangNghia17/ChangeLogin_Account_RiotClient/releases)
2. Extract the ZIP file
3. Run `RiotAccountManager.exe` (double-click)

**Note:** NO Java required. Windows 10/11 with WebView2 (preinstalled on most PCs).

## Usage Guide

### Step 1: Configure Riot Client
1. Open the application
2. Click "Browse" button to select path to `RiotClientServices.exe`
3. Default path is usually: `C:\Riot Games\Riot Client\RiotClientServices.exe`

### Step 2: Add Account
1. Click "Add Account" button (person with + icon)
2. Enter Username, Password, and select Region
3. (Optional) Add Note to distinguish accounts
4. Click "Save"

### Step 3: Login
1. Make sure Riot Client is open (click "Open Riot Client" if not)
2. Select account from the list
3. Click "Login" button (lock icon)
4. Application will automatically fill login information
5. Check information and press Enter to login

### Manage Accounts
- **Edit**: Select account and click "Edit" button (pencil icon)
- **Delete**: Select account and click "Delete" button (trash icon)

## System Requirements

- Windows 10/11
- Riot Client installed
- No Java required (native Tauri app)
- WebView2 (included on Windows 10/11)

## Support

If you encounter issues, please:
1. Check if Riot Client is open
2. Check if Riot Client path is correct
3. Make sure Riot Client is showing login screen
4. Create an [issue on GitHub](https://github.com/DangNghia17/ChangeLogin_Account_RiotClient/issues) if problem persists

## License

MIT License - See LICENSE file for more details.

## Author

Riot Account Manager - Open source, free to use.

**GitHub:** [https://github.com/DangNghia17/ChangeLogin_Account_RiotClient](https://github.com/DangNghia17/ChangeLogin_Account_RiotClient)

