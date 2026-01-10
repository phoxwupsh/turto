# turto

[English](https://github.com/phoxwupsh/turto/blob/main/README.md) | [繁體中文](https://github.com/phoxwupsh/turto/blob/main/README.zh-Hant.md)

turto 是一個簡單易用、可高度自訂的 Discord 音樂機器人，特別適合自架。跟著下面步驟，幾分鐘內就能擁有自己的 turto。

## 特點

- 無需設定，開箱即用
- 完全可自訂的多國語言支援
- 5 分鐘內完成部署
- 支援市面上幾乎所有串流平台（由 yt-dlp 驅動）
  - 自動管理 yt-dlp 版本
- 斜線指令

## ⚠️從舊版本升級

如果你是從 0.x.x 版本升級，你可能需要先把機器人從伺服器踢掉，並依照下方步驟重新邀請一次。

## 部署

> [!TIP]
> turto 也支援 Docker，如果你想用 Docker 執行 turto，請看[這裡](https://github.com/phoxwupsh/turto#use-turto-with-docker)。

### 下載執行檔

你可以從 [release 頁面](https://github.com/phoxwupsh/turto/releases)下載我已編譯好的 turto 執行檔。如果你使用的平台沒有提供，也可以自行編譯。

#### .env

下載執行檔後，你需要在同一個資料夾旁建立 `.env` 檔案，並用文字編輯器寫入以下內容：

```
DISCORD_TOKEN=<你的 Discord token>
```

你需要把你的 **token** 貼在 `DISCORD_TOKEN=` 後面並存檔。如果你不知道什麼是 Token，可以直接 [搜尋「discord bot token」](https://www.google.com/search?q=discord+bot+token)，會有很多教學。你也需要在取得 Token 的同一頁面把 **MESSAGE CONTENT INTENT** 打開。

> [!NOTE]
> 實際上我們需要的是把環境變數 `DISCORD_TOKEN` 設為你的 token，你可以用任何方式達成。

### 啟動機器人

turto 除了 token 以外不需要任何設定就能運作，但你仍然可以調整許多設定。
若要用全預設值直接執行，只要不帶任何參數啟動即可。

#### Windows

在 Windows 你可以直接點兩下 `turto.exe` 啟動，或使用命令提示字元 / PowerShell：

```shell
.\turto
```

#### Linux/macOS

確認執行檔有執行權限（若沒有可用以下指令設定）：

```shell
chmod +x turto
```

然後啟動：

```shell
./turto
```

### 啟動參數

你可以用 `-?` 或 `--usage` 查看參數，完整用法如下：

```
Usage: turto [OPTIONS]

Options:
  -?, --usage             show the usage
      --config <FILE>     path to config file [default: config.toml]
      --guilds <FILE>     path to guilds data file [default: guilds.json]
      --help <FILE>       path to help messages file [default: help.toml]
      --tempaltes <FILE>  path to message templates file [default: templates.toml]
```

你可以用像 `--config path/to/your/config.toml` 或 `--guilds path/to/your/guilds.json` 這種方式指定要使用的設定/資料檔。

> [!NOTE]
> 若沒有指定設定檔，turto 會嘗試使用同目錄下的設定檔預設名稱檔案。

## 設定

turto 可以在沒有設定檔的情況下運作，但你仍可以透過一些 TOML 檔進行調整。
我們提供包含 `zh-TW` 語言支援的範例檔，你可以直接複製後再依需求修改。

- [`config.toml`](https://github.com/phoxwupsh/turto/blob/main/config.example.toml)
- [`help.toml`](https://github.com/phoxwupsh/turto/blob/main/help.example.toml)
- [`template.toml`](https://github.com/phoxwupsh/turto/blob/main/template.example.toml)

> [!TIP]
> 編輯時請確保符合 [TOML 格式](https://toml.io/en/v1.0.0)。

### 基礎設定

你可以參考 [`config.toml`](https://github.com/phoxwupsh/turto/blob/main/config.example.toml) 的範例檔，用來調整 turto 的常見行為。
每個參數的用途都寫在檔案內的註解中。

> [!TIP]
> - `cookies_path` 可以使用像 [Get cookies.txt LOCALLY](https://chromewebstore.google.com/detail/get-cookiestxt-locally/cclelndahbckbenkjhflpdbgdldlbecc) 之類的擴充功能取得 cookies（需為 Netscape 格式的文字檔）。
> - 若設定了 `owner`，owner 可以在不需要管理員權限的情況下使用 `/ban` 與 `/unban` 指令，且 `/about` 會顯示 turto 屬於誰。

### 多國語言支援

turto 支援多國語言，會依照使用者的 locale 設定顯示對應語言回應，預設語言為英文。
若要新增語言或修改回應內容，可參考 [`help.toml`](https://github.com/phoxwupsh/turto/blob/main/help.example.toml) 與 [`template.toml`](https://github.com/phoxwupsh/turto/blob/main/template.example.toml)。
你可以編輯 `default` 區塊，讓 turto 使用不同的預設語言（或任何你想讓它說的內容），也可以新增像 `zh-TW` 這樣的區塊來支援其他語言。
turto 會優先使用有支援的語言；若使用者語言未被支援，則會使用 `default` 區塊的內容回應。
更詳細的設定方式都寫在 `help.toml` 與 `template.toml` 的註解中。

### 邀請 turto 到你的伺服器

你可以在 [Discord Developer Portal](https://discord.com/developers/applications) → Applications → *你的 bot 應用程式* → General Information 找到 application ID，並把下方網址中的 `your_application_id` 替換成你的 application ID：

```
https://discord.com/api/oauth2/authorize?client_id=<your_application_id>&permissions=36718592&scope=bot+applications.commands
```

或是在 Discord Developer Portal → Applications → *你的 bot 應用程式* → OAuth2 → URL Generator 自行產生網址。請勾選以下選項：

#### Scopes
- bot
- applications.commands

#### Bot permissions
- Send Messages
- Embed Links
- Connect
- Speak
- Use Voice Activity

## 使用方法

你只要輸入 `/` 就能看到 turto 的所有指令列表。斜線指令本身會說明指令與參數；若要更詳細資訊，可使用 `/help` 查詢每個指令的用法。

## 關閉

如果你想停止 bot，請按 `Ctrl` + `C`，turto 會先儲存各伺服器資料（例如播放清單與設定）再關閉。若你直接關閉終端機視窗，資料可能不會被儲存。

## 除錯模式

除錯模式會顯示更多程式資訊，方便除錯；若你準備回報 bug，建議先啟用除錯模式並提供 log。

### Windows

#### PowerShell

```powershell
$env:TURTO_LOG="debug"
.\turto
```

#### Command Prompt

```batch
set TURTO_LOG=debug
.\turto
```

### Linux/macOS

```shell
export TURTO_LOG=debug
./turto
```

## 用 Docker 執行 turto

要以最簡單的方式啟動 turto，你可以直接執行：

```shell
docker run -e DISCORD_TOKEN=your_bot_token ghcr.io/phoxwupsh/turto:latest
```

把 `your_bot_token` 替換成你的 Discord bot token 即可。

### 在 Docker 中設定 turto

要在 Docker 中設定 turto，需要使用 Docker volume：先把設定檔放在宿主機任意位置並修改，再像這樣掛載：

```shell
docker run \
  -e DISCORD_TOKEN=your_bot_token \
  -v /path/to/your/config.toml:/app/config.toml \
  -v /path/to/your/help.toml:/app/help.toml \
  -v /path/to/your/templates.toml:/app/templates.toml \
  ghcr.io/phoxwupsh/turto:latest
```

其中 `/path/to/your/config.toml` 是宿主機上的 `config.toml`，`help.toml` 與 `tempaltes.toml` 同理；
你只要在宿主機編輯它們，Docker 容器啟動後就會套用。

你也可以把資料存放在宿主機上，只要掛載 `guilds.json`：

```shell
docker run \
  -e DISCORD_TOKEN=your_bot_token \
  -v /path/to/your/guilds.json:/app/guilds.json \
  ghcr.io/phoxwupsh/turto:latest
```

## 編譯

要編譯 turto，你需要 Rust toolchain 與 CMake。

### Rust toolchain

要安裝 Rust toolchain，請參考[這裡](https://www.rust-lang.org/tools/install)並依指示安裝。

安裝完成後，請確認 Rust 版本高於 `1.88.0`，你可以用以下指令查看版本：

```shell
rustc -V
```

### CMake

你可以從 [CMake 官網](https://cmake.org/download/) 下載對應平台的 CMake。

#### 原始碼

確保已安裝 Rust toolchain 與 CMake 後，接著需要下載原始碼。你可能會想 fork 一份（尤其如果你打算貢獻），或直接用 Git 下載：

```shell
git clone https://github.com/phoxwupsh/turto.git
```

### 開始編譯

進到 `Cargo.toml` 所在資料夾：

```shell
cd turto
```

開始編譯：

```shell
cargo build --release
```

編譯成功後，你可以在 `target` → `release` 資料夾找到 turto 執行檔。
