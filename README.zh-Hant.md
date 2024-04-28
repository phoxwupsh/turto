# turto

[English](https://github.com/phoxwupsh/turto/blob/main/README.md) | [繁體中文](https://github.com/phoxwupsh/turto/blob/main/README.zh-Hant.md)

turto是一個簡單易用而且支援許多自定義選項的Discord音樂機器人，很適合想要自己架設Discord音樂機器人的你。只要跟著下面的指示，不用幾分鐘你就能擁有自己的turto。

## 特點

- 完全可以自定義的多國語言支援
- 5分鐘內可以完成架設
- 支援市面上幾乎所有影音平台(透過yt-dlp來實現這件事)
- 使用斜線指令

## ⚠️更新注意事項

如果你是從舊版的0.x.x的舊版本更新上來的，那你可能需要將原本的機器人踢出你的伺服器，再依照下面的步驟重新邀請一次。

## 部署

turto需要yt-dlp才能運作，如果你還沒有安裝它的話，你可以照著下面的指示來安裝。turto也支援Docker，如果你要用Docker來執行turto的話，可以直接跳過以下這些步驟看[這裡](https://github.com/phoxwupsh/turto/blob/main/README.zh-Hant.md#%E7%94%A8Docker%E5%AE%B9%E5%99%A8%E4%BE%86%E5%9F%B7%E8%A1%8Cturto)。

### yt-dlp

在yt-dlp他們Github上的[這個Wiki頁面](https://github.com/yt-dlp/yt-dlp/wiki/Installation)有詳盡的指示教你如何在各種平台上安裝yt-dlp。

### 下載執行檔

在你確定yt-dlp安裝好之後，你可以從[這裡](https://github.com/phoxwupsh/turto/releases)按照你所使用的平台下載我已經預先編譯好的turto版本。如果我沒有幫你正在使用的平台編譯的話，你也可以自己編譯。

#### .env

先將你下載到的壓縮檔解壓縮，你會在資料夾中看到`.env`這個檔案，用記事本之類的文字編輯器開啟這個檔案你會看到像這樣

```
DISCORD_TOKEN=
```
你需要把你的**Token**貼在`DISCORD_TOKEN=`後面然後存檔。如果你還沒有Token的話，可以直接[Google搜尋「discord 機器人 token」](https://www.google.com/search?q=discord+%E6%A9%9F%E5%99%A8%E4%BA%BA+token)然後你就會看到一堆教學。並且，你也需要把**MESSAGE CONTENT INTENT**選項打開，這個選項在你取得Token的那個地方的下面。

### 啟動

#### Windows

在Windows中你只要點兩下`turto.exe`它就會啟動了，或是你也可以用命令提示字元或Powershell來啟動它。

```shell
.\turto
```

#### Linux/macOS

記得要先幫執行檔設定執行權限

```shell
chmod +x turto
```
然後再啟動

```shell
./turto
```

## 設定

turto執行檔所在的資料夾下有`config.toml`、`help.toml`跟`template.toml`等檔案，可以用記事本打開它們來修改turto的各項設定，由於是用TOML格式紀錄，編輯時記得要按照[TOML的格式](https://toml.io/en/v1.0.0)。

### 基礎設定

`config.toml`用於turto的設定，每個參數的用途已經寫在檔案的註解中。`owner`則不一定要設定，如果有設定的話，owner能無視管理員權限使用`/ban`、`/unban`指令，並且在`/about`指令中會顯示這個turto屬於誰。

### 多國語言設定

turto支援多國語言，會依照使用者的地區設定來顯示對應語言的回應，預設支援英文與繁體中文。需要新增語言支援或是修改回應的話可以編輯`help.toml`跟`template.toml`這兩個檔案，在這兩個檔案中你會看到有`default`的區塊以及`zh-TW`的區塊，turto會優先使用有支援的語言，如果沒有支援該語言則會使用`default`的內容來回應。你也可以以編輯`default`區塊的內容讓turto預設使用其他種語言(或是任何你想讓它說的話)，詳細的設定說明都寫在`help.toml`跟`template.toml`的註解中。

### 邀請turto到你的伺服器

你可以去Discord的[Developer Portal](https://discord.com/developers/applications) &rarr; Applications &rarr; *你的機器人的應用程式* &rarr; General Information找到你的應用程式ID，然後將下面這個網址中的`your_application_id`換成你的應用程式ID

```
https://discord.com/api/oauth2/authorize?client_id=your_application_id&permissions=36718592&scope=bot+applications.commands
```

然後你就可以用這個網址邀請turto到你的伺服器了。或是你也可以自己去Discord的Developer Portal &rarr; Applications &rarr; *你的機器人的應用程式* &rarr; OAuth2 &rarr; URL Generator來生成網址，記得要勾選以下的選項

**Scopes**
- bot
- applications.commands

**Bot permissions**
- Send Messages
- Embed Links
- Connect
- Speak
- Use Voice Activity

## 使用方法

你只需要打一個`/`就能看到turto所有指令的列表，斜線指令本身有針對指令及參數的說明，如果需要更詳細的資訊可以使用`/help`指令來查詢每個指令的詳細用法。

## 關閉

當你要讓turto停止運作的時候，你要在終端機視窗中按下`Ctrl`和`C`，這樣turto才會把資料(例如播放清單和各伺服器設定)儲存起來，反之如果你直接把視窗關掉那資料就不會儲存。

## 除錯模式

除錯模式會顯示更多程式運作的資訊，建議在回報bug前先啟用除錯模式，並提供log記錄檔。

### Windows

**PowerShell**
```powershell
$env:TURTO_LOG="debug"
.\turto
```
**Command Prompt**
```batch
set TURTO_LOG=debug
.\turto
```

### Linux/macOS
```shell
export TURTO_LOG=debug
./turto
```

## 用Docker容器來執行turto

你只需要執行

```shell
docker run -e DISCORD_TOKEN=your_bot_token ghcr.io/phoxwupsh/turto:latest
```

記得用你自己的Token取代`your_bot_token`，就這麼簡單。

## 編譯

要自己編譯turto你會需要Rust工具鏈和CMake。

### Rust開發工具

你可以直接去[Rust的官網](https://www.rust-lang.org/tools/install)按照裡面的指示安裝。

安裝好之後你可以用下面這個指令確定你的Rust版本至少有`1.74.0`。

```shell
rustc -V
```

### CMake

你可以到[CMake的官網](https://cmake.org/download/)下載對應平台的CMake。

#### turto原始碼

安裝好Rust開發工具和CMake之後，你需要下載turto的原始碼，你可以fork一份自己的(尤其如果你有打算要貢獻)原始碼，或是直接

```shell
git clone https://github.com/phoxwupsh/turto.git
```

### 開始編譯

先進到`Cargo.toml`所在的資料夾

```shell
cd turto
```

然後就可以開始編譯了

```shell
cargo build --release
```

編譯完成後，在`target` &rarr; `release`資料夾中你就可以找到turto的執行檔。如果你是自己編譯的話，你還是會需要將`.env`、`config.toml`、`help.toml`和`templates.toml`等檔案放入執行檔所所在的資料夾。你可以找到以`.template`結尾的對應檔案，只要將它們重新命名即可。