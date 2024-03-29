# turto

[English](https://github.com/phoxwupsh/turto/blob/main/README.md) | [繁體中文](https://github.com/phoxwupsh/turto/blob/main/README.zh-Hant.md)

turto is a simple, easy-to-use and customizable Discord music bot, especially suitable for self-hosting. Through the following steps, you can have your own turto in minutes.

## Deployment

turto depends on yt-dlp, make sure you have it installed, if you haven't done so, you can follow the following steps to install it. turto also supports Docker, see [here](https://github.com/phoxwupsh/turto#if-you-use-docker) if you want to use turto with Docker.

### yt-dlp

For installing yt-dlp, you can refer to [this page on their github repository](https://github.com/yt-dlp/yt-dlp/wiki/Installation), they have made instructions for various platforms.

### Setup the bot

Since you ensure that yt-dlp is installed, you can download pre-compiled turto binaries from the [release page](https://github.com/phoxwupsh/turto/releases). If the platform that you're using isn't provided, you can also compile it yourself.

#### .env

After you extract the zip file downloaded from the release page, you will see `.env` file, open it with text editor then you will see the content is like below.

```
DISCORD_TOKEN=
```
You need to paste you **Token** right after `DISCORD_TOKEN=`, and save the file. If you don't know what is Token, you can just [seach "discord bot token"](https://www.google.com/search?q=discord+bot+token) and there are a lot of tutorials telling how to do. You also need to turn the **MESSAGE CONTENT INTENT** on, in the same page as you get Token.

#### config.toml

In the same directory there is also `config.toml`, it's configuration file for the bot. You can edit it with text editor, basically each attributes is just like what the comment said, just make sure you follow the [TOML specification](https://toml.io/en/v1.0.0). The `owner` attribute is optional, the bot can still work if you don't set it. If `owner` is set, the owner can use `ban` and `unban` commands without administrator permission, and the `about` command will show that this turto is owned by who.

### Launch the bot

#### Windows

On Windows you can simply double click the `turto.exe` file then the bot will start working, or you can also use Command Prompt or PowerShell.

```shell
.\turto
```

#### Linux/macOS

Make sure the exetuable have execute permission, if not, it can be done by

```shell
chmod +x turto
```
And you can start the bot by

```shell
./turto
```

### Invite the bot to your guild

You can get your application ID in [Discord Developer Portal](https://discord.com/developers/applications) &rarr; Applications &rarr; *Your bot's application* &rarr; General Information, and replace `your_application_id` in the URL below.

```
https://discord.com/api/oauth2/authorize?client_id=your_application_id&permissions=36727808&scope=bot
```

Or, you can generate the URL in Discord Developer Portal &rarr; Applications &rarr; *Your bot's application* &rarr; OAuth2 &rarr; URL Generator. Make sure to select these following options.

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

### Customize

In the directory where the bot executable is, there are two files `help.toml` and `template.toml`, you can customize almost every message that the bot will send in these two files, by just simply edit the file with text edtior. For more detail, you can refer to comments in the file.

Basically there's no need to edit these file, since I have written presets for you.

### Usage

Basically you can get instructions of each command using the `help` command, there are also some example included. Make sure to add the prefix (`command_prefix`) you set in `config.toml` before using commands.

### Shutdown

If you want to stop the bot, you should press `Ctrl` + `C`, this make the bot save data of each guilds (playlist and settings) then shutdown. If you close the terminal window directly, the bot would not save the data.

### If you use Docker

Run this command

```shell
docker run -e DISCORD_TOKEN=your_bot_token ghcr.io/phoxwupsh/turto:latest
```

You need to replace `your_bot_token` with your Discord bot token, that's it.

## Compile

To compile turto, you will need Rust toolchain and CMake.

### Rust toolchain

To install Rust toolchain, you can visit [here](https://www.rust-lang.org/tools/install), and follow the instruction.

After you finish the installation, make sure your Rust version is higher than `1.74.0`, you can check your Rust version by

```shell
rustc -V
```

### CMake

#### Windows

If you are using Scoop, Chocolatey or winget, you can install it with them. Or, you can also download the installer [here](https://cmake.org/download/).

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

You can install CMake on Linux using package manager. Depends on what distribution you are using, below are commands for some common package managers. Or, you can also download the installer [here](https://cmake.org/download/).

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

You can install FFmpeg on macOS using Homebrew package manager. Or, you can also download the installer [here](https://cmake.org/download/).

```shell
brew install cmake
```

#### Get source code

Since you ensure Rust toolchain and CMake are installed, you need to to clone this repository with Git. Or, you can directly download from github thorugh the release page or the Download ZIP.

```shell
git clone https://github.com/phoxwupsh/turto.git
```

### Start compiling

Then you can head to the directory where `Cargo.toml` is

```shell
cd turto
```

And start compiling

```shell
cargo build --release
```

After it compile successfully, you can see turto executable in directory `target` &rarr; `release`. If you compile turto yourself, you will need `.env`, `config.toml`, `help.toml` and `templates.toml` in the same directory with the executable, you can find presets in this repository, with file name end with `.template`, you can simply rename them and start using.
