# turto

[English](https://github.com/phoxwupsh/turto/blob/main/README.md) | [繁體中文](https://github.com/phoxwupsh/turto/blob/main/README.zh-Hant.md)

turto是一個簡單易用而且支援許多自定義選項的Discord音樂機器人，很適合想要自己架設Discord音樂機器人的你。只要跟著下面的指示，不用幾分鐘你就能擁有自己的turto。

## 部署

turto需要yt-dlp才能運作，如果你還沒有安裝它的話，你可以照著下面的指示來安裝。turto也支援Docker，如果你要用Docker來執行turto的話，可以直接跳過以下這些步驟看[這裡](https://github.com/phoxwupsh/turto/blob/main/README.zh-Hant.md#%E5%A6%82%E6%9E%9C%E4%BD%A0%E6%98%AF%E7%94%A8docker%E7%9A%84%E8%A9%B1)。

### yt-dlp

在yt-dlp他們Github上的[這個Wiki頁面](https://github.com/yt-dlp/yt-dlp/wiki/Installation)有詳盡的指示教你如何在各種平台上安裝yt-dlp。

### 設定

在你確定yt-dlp安裝好之後，你可以從[這裡](https://github.com/phoxwupsh/turto/releases)按照你所使用的平台下載我已經預先編譯好的turto版本。如果我沒有幫你正在使用的平台編譯的話，你也可以自己編譯。裡面預設的檔案是英文的，如果你希望turto說中文，可以在同一個頁面下載`zh-Hant.zip`，並用`zh-Hant.zip`中的`help.toml`和`templates.toml`取代原本的檔案。

#### .env

先將你下載到的壓縮檔解壓縮，你會在資料夾中看到`.env`這個檔案，用記事本之類的文字編輯器開啟這個檔案你會看到像這樣

```
DISCORD_TOKEN=
```
你需要把你的**Token**貼在`DISCORD_TOKEN=`後面然後存檔。如果你還沒有Token的話，可以直接[Google搜尋「discord 機器人 token」](https://www.google.com/search?q=discord+%E6%A9%9F%E5%99%A8%E4%BA%BA+token)然後你就會看到一堆教學。並且，你也需要把**MESSAGE CONTENT INTENT**選項打開，這個選項在你取得Token的那個地方的下面。

#### config.toml

在同一個資料夾下你會看到`config.toml`這個檔案，它是turto的設定檔案，要編輯的話可以用記事本打開檔案，基本上每個參數代表的意思已經寫在檔案的註解中，記得編輯的時候要符合[TOML的規定](https://toml.io/en/v1.0.0)。`owner`不一定要設定，沒設定turto也能照常運作。有設定的話，owner能無視管理員權限使用`ban`、`unban`指令，並且`在about`指令中會顯示這個turto屬於誰。

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

### 把turto邀請到你的伺服器

你可以去Discord的[Developer Portal](https://discord.com/developers/applications) &rarr; Applications &rarr; *你的機器人的應用程式* &rarr; General Information找到你的應用程式ID，然後用你的應用程式ID取代下面這個網址中的`your_application_id`。

```
https://discord.com/api/oauth2/authorize?client_id=your_application_id&permissions=36727808&scope=bot
```

或是你也可以自己去Discord的Developer Portal &rarr; Applications &rarr; *你的機器人的應用程式* &rarr; OAuth2 &rarr; URL Generator來生成網址，記得要勾選以下的選項

**Scopes**
- bot

**Bot permissions**
- Read Messages/View Channels
- Send Messages
- Manage Messages
- Embed Links
- Connect
- Speak
- Use Voice Activity

### 自定義

在跟執行檔同一個資料夾下還有`help.toml`跟`template.toml`這兩個檔案，你可以用記事本打開這兩個檔案來自訂turto會傳出的幾乎所有的訊息，檔案的註解中有寫每個參數跟訊息是什麼意思。

基本上你也不用自己寫，因為我都幫你寫好了。

### 使用方式

所有指令的使用方式都寫在`help`指令中了，你可以直接使用`help`指令來查詢每個指令的用法和使用範例。記得使用每個指令要在前面加上你在`config.toml`中設定過的`command_prefix`。

### 關閉

當你要讓turto關機的時候，你要按`Ctrl`和`C`，這樣turto才會把資料(例如播放清單和設定)儲存起來，反之如果你直接把視窗關掉那資料就不會儲存。

### 如果你是用Docker的話

你只需要執行

```shell
docker run -e DISCORD_TOKEN=your_bot_token ghcr.io/phoxwupsh/turto:latest
```

記得用你自己的Token取代`your_bot_token`，就這麼簡單。

## 編譯

要自己編譯turto你會需要Rust工具鏈和CMake。

### Rust工具鏈

你可以直接去[他們的網站](https://www.rust-lang.org/tools/install)按照裡面的指示安裝。

安裝好之後你可以用下面這個指令確定你的Rust版本至少有`1.74.0`。

```shell
rustc -V
```

### CMake

#### Windows

如果你有在用Scoop、Chocolatey或winget，就直接用它們安裝就好了。或是你也可以到[這裡](https://cmake.org/download/)下載。

##### Scoop
```shell
scoop install cmake
```

##### Chocolatey
```shell
choco install cmake
```

##### winget
```shell
winget install --id=Kitware.CMake -e
```

#### Linux

在Linux上你可以直接用內建的套件管理工具來安裝CMake，以下是幾個較常見Linux發行版的安裝方式。或是你也可以到[這裡](https://cmake.org/download/)下載。

##### Debian/Ubuntu:
```shell
sudo apt-get install cmake
```

##### Fedora
```shell
sudo dnf install cmake
```

##### Arch Linux
```shell
sudo pacman -S cmake
```

#### macOS

在mac上你可以用Homebrew套件管理工具來安裝CMake。或是你也可以到[這裡](https://cmake.org/download/)下載。

```shell
brew install cmake
```

#### Get source code

安裝好Rust工具鏈和CMake之後，你需要下載turto的原始碼，你可以用Git或是去[這裡](https://github.com/phoxwupsh/turto/releases)下載。

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

編譯完成後，在`target` &rarr; `release`資料夾中你就可以找到turto的執行檔。如果你是自己編譯的話，你還是會需要將`.env`、`config.toml`、`help.toml`和`templates.toml`等檔案放入執行檔所所在的資料夾，你可以在原始碼中找到以`.template`結尾的檔案，重新命名即可使用(`.template`前有`zh-Hant`的是中文版本)。