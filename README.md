# turto

[English](https://github.com/phoxwupsh/turto/blob/main/README.md) | [繁體中文](https://github.com/phoxwupsh/turto/blob/main/README.zh-Hant.md)

turto is a simple, easy-to-use and customizable Discord music bot, especially suitable for self-hosting. Through the following steps, you can have your own turto in minutes.

## Features

- Fully customizable multilingual support
- Deploy within only 5 minutes
- Support almost all platform (powered by yt-dlp)
- Slash commands

## ⚠️Upgrade from older version

If you are upgrading from version 0.x.x, you might need to kick the bot from your server, and re-invite with steps below.

## Deployment

turto depends on yt-dlp, make sure you have it installed, if you haven't done so, you can follow these steps to install it. turto also supports Docker, see [here](https://github.com/phoxwupsh/turto#use-turto-with-docker) if you want to use turto with Docker.

### yt-dlp

For installing yt-dlp, you can refer to [this page on their github repository](https://github.com/yt-dlp/yt-dlp/wiki/Installation), they have made instructions for various platforms.

### Download the executable

Since you ensure that yt-dlp is installed, you can download pre-compiled turto binaries from the [release page](https://github.com/phoxwupsh/turto/releases). If the platform that you're using isn't provided, you can also compile it yourself.

#### .env

After you extract the zip file downloaded from the release page, you will see `.env` file, open it with text editor then you will see the content is like below.

```
DISCORD_TOKEN=
```
You need to paste you **Token** right after `DISCORD_TOKEN=`, and save the file. If you don't know what is Token, you can just [seach "discord bot token"](https://www.google.com/search?q=discord+bot+token) and there are a lot of tutorials telling how to do. You also need to turn the **MESSAGE CONTENT INTENT** on, in the same page as you get Token.

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

## Configuration

In the same folder as the turto executable, you'll find files like `config.toml`, `help.toml`, and `template.toml`. You can open these files in text editors to tweak turto's settings. Since they use the TOML format, make sure you edit them according to the [TOML formats](https://toml.io/en/v1.0.0).

### Basic configuration

`config.toml` is used for configuring turto, the purpose of each parameter is described in the comments within the file. The `owner` parameter does not necessarily need to be set, but if it is, the owner has the ability to bypass admin permissions to use the `/ban` and `/unban` commands, and the `/about` command will display who turto belongs to.

### Multilingual Support
turto supports multiple languages and will display responses in the language corresponding to the user's regional settings, with English and Traditional Chinese being the default supported languages. To add support for new languages or to modify responses, you can edit `help.toml` and `template.toml`. In these files, you will see sections like `default` and `zh-TW`. turto will prioritize using a supported language, but if there’s no support for the user's language, it will respond with the content from the default section. You can also edit the content of the default section to have turto use a different default language (or whatever you want it to say). Detailed configuration instructions are written in the comments of both `help.toml` and `template.toml`.

### Invite the bot to your guild

You can get your application ID in [Discord Developer Portal](https://discord.com/developers/applications) &rarr; Applications &rarr; *Your bot's application* &rarr; General Information, and replace `your_application_id` in the URL below.

```
https://discord.com/api/oauth2/authorize?client_id=your_application_id&permissions=36718592&scope=bot+applications.commands
```

Or, you can generate the URL in Discord Developer Portal &rarr; Applications &rarr; *Your bot's application* &rarr; OAuth2 &rarr; URL Generator. Make sure to select these following options.

**Scopes**
- bot
- applications.commands

**Bot permissions**
- Send Messages
- Embed Links
- Connect
- Speak
- Use Voice Activity

## Usage

All you need to do is type `/` to see a list of all commands available in turto. The slash command provides explanations for both the commands and their parameters. For more detailed information, you can use the `/help` command to inquire about the specific usage of each command.

## Shutdown

If you want to stop the bot, you should press `Ctrl` + `C`, this make the bot save data of each guilds (like playlist and settings) then shutdown. If you close the terminal window directly, the bot would not save the data.

## Debug mode

Debug mode shows more information of the program, which is useful for debugging, if you are about to report bugs, it would be better to enable debug mode and provide logs.

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

## Use turto with Docker

Run this command

```shell
docker run -e DISCORD_TOKEN=your_bot_token ghcr.io/phoxwupsh/turto:latest
```

You need to replace `your_bot_token` with your Discord bot token, that's it.

## Compile

To compile turto, you will need Rust toolchain and CMake.

### Rust toolchain

To install Rust toolchain, you can visit [here](https://www.rust-lang.org/tools/install), and follow the instruction.

After you finish the installation, make sure your Rust version is higher than `1.80.0`, you can check your Rust version by

```shell
rustc -V
```

### CMake

You can download CMake for your platform [from their website](https://cmake.org/download/).

#### Source code

Since you ensure Rust toolchain and CMake are installed, you need to download the source. You might want to this repository, especially if you're planning on contributing. Or, you can directly download with Git.

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
