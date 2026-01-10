# turto

[English](https://github.com/phoxwupsh/turto/blob/main/README.md) | [繁體中文](https://github.com/phoxwupsh/turto/blob/main/README.zh-Hant.md)

turto is a simple, easy-to-use and customizable Discord music bot, especially suitable for self-hosting. Through the following steps, you can have your own turto in minutes.

## Features

- Zero configuration, out of box
- Fully customizable multilingual support
- Deploy within only 5 minutes
- Support almost all streaming platform (powered by yt-dlp)
  - Automatic yt-dlp version management
- Slash commands

## ⚠️Upgrade from older version

If you are upgrading from version 0.x.x, you might need to kick the bot from your server, and re-invite with steps below.

## Deployment

> [!TIP]
> turto also supports Docker, see [here](https://github.com/phoxwupsh/turto#use-turto-with-docker) if you want to use turto with Docker.

### Download the executable

You can download pre-compiled turto executables from the [release page](https://github.com/phoxwupsh/turto/releases). If the platform that you're using isn't provided, you can also compile it yourself.

#### .env

After you downloaded the executables, you will need to create a `.env` file alongside with, write the below content in it with text editor.

```
DISCORD_TOKEN=<your Discord token>
```

You need to paste you **token** right after `DISCORD_TOKEN=`, and save the file. If you don't know what is Token, you can just [seach "discord bot token"](https://www.google.com/search?q=discord+bot+token) and there are a lot of tutorials telling how to do. You also need to turn the **MESSAGE CONTENT INTENT** on, in the same page as you get token.

> [!NOTE]
> Actually what we need is the environment variable `DISCORD_TOKEN` to be set to your token, you can achieve this is by any kind of method.

### Launch the bot

turto can run without any configuration or settings (except token), but there are still many configuration you can tweak.
To simply run the turto without any configuration (will use all default) you can simply run with no option.

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

### Launch options

You can run with `-?` or `--usage` to see the options, below is the full usage

```
Usage: turto [OPTIONS]

Options:
  -?, --usage             show the usage
      --config <FILE>     path to config file [default: config.toml]
      --guilds <FILE>     path to guilds data file [default: guilds.json]
      --help <FILE>       path to help messages file [default: help.toml]
      --tempaltes <FILE>  path to message templates file [default: templates.toml]
```

> [!NOTE]
> If configuration files are not specified, turto will try to use the files with default names in the same directory.

You can specify the configuration or data files you want to use with options like `--config path/to/your/config.toml` or `--guilds path/to/your/guilds.json`.

## Configuration

turto can work without configuration, but you can still tweak turto with some TOML files. 
We provide some example for the configuration files, which include `zh-TW` language support, you can simply copy them and modify for your needs.

- [`config.toml`](https://github.com/phoxwupsh/turto/blob/main/config.example.toml)
- [`help.toml`](https://github.com/phoxwupsh/turto/blob/main/help.example.toml)
- [`template.toml`](https://github.com/phoxwupsh/turto/blob/main/template.example.toml)

> [!TIP]
> Make sure you edit them according to the [TOML formats](https://toml.io/en/v1.0.0).

### Basic configuration

You can reference to example file of [`config.toml`](https://github.com/phoxwupsh/turto/blob/main/config.example.toml), which is used for tweaking common turto behaviors. 
The purpose of each attribute is described in the comments within the file.

> [!TIP]
> - For `cookies_path`, you can get cookies by using extension like [Get cookies.txt LOCALLY](https://chromewebstore.google.com/detail/get-cookiestxt-locally/cclelndahbckbenkjhflpdbgdldlbecc), which should be a Netscape format text file.
> - If `owner` is set, the owner has the ability to bypass admin permissions to use the `/ban` and `/unban` commands, and the `/about` command will display who turto belongs to.

### Multilingual Support
turto supports multiple languages and will display responses in the language corresponding to the user's locale settings, with English being the default supported languages.
To add support for new languages or to modify responses, you can reference to [`help.toml`](https://github.com/phoxwupsh/turto/blob/main/help.example.toml) and [`template.toml`](https://github.com/phoxwupsh/turto/blob/main/template.example.toml).
You can add and edit the `default` section  to have turto use a different default language (or whatever you want it to say), you can add other section like `zh-TW` for other languages.
turto will prioritize using a supported language, and respond with the content from the default section if there’s no support for the user's language. 
Detailed configuration instructions are written in the comments of both `help.toml` and `template.toml`.

### Invite the bot to your guild

You can get your application ID in [Discord Developer Portal](https://discord.com/developers/applications) &rarr; Applications &rarr; *Your bot's application* &rarr; General Information, and replace `your_application_id` in the URL below.

```
https://discord.com/api/oauth2/authorize?client_id=<your_application_id>&permissions=36718592&scope=bot+applications.commands
```

Or, you can generate the URL in Discord Developer Portal &rarr; Applications &rarr; *Your bot's application* &rarr; OAuth2 &rarr; URL Generator. Make sure to select these following options.

#### Scopes
- bot
- applications.commands

#### Bot permissions
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

## Use turto with Docker

To startup turto with minimal setup, you can simply run this command

```shell
docker run -e DISCORD_TOKEN=your_bot_token ghcr.io/phoxwupsh/turto:latest
```

You need to replace `your_bot_token` with your Discord bot token, that's it.

### Configure turto in Docker

To configure turto in Docker we need to setup Docker volume, we place and modify the configuration files somethere in your host, then mount them like this:

```shell
docker run \
  -e DISCORD_TOKEN=your_bot_token \
  -v /path/to/your/config.toml:/app/config.toml \
  -v /path/to/your/help.toml:/app/help.toml \
  -v /path/to/your/templates.toml:/app/templates.toml \
  ghcr.io/phoxwupsh/turto:latest
```

That `/path/to/your/config.toml` is  the `config.toml` in your host, the same goes for `help.toml` and `tempaltes.toml`, 
then you can edit them in your host, which will be applied after the Docker container starts.

You can also store the data in your host, by mounting `guilds.json`

```shell
docker run \
  -e DISCORD_TOKEN=your_bot_token \
  -v /path/to/your/guilds.json:/app/guilds.json \
  ghcr.io/phoxwupsh/turto:latest
```

## Compile

To compile turto, you will need Rust toolchain and CMake.

### Rust toolchain

To install Rust toolchain, you can visit [here](https://www.rust-lang.org/tools/install), and follow the instruction.

After you finish the installation, make sure your Rust version is higher than `1.88.0`, you can check your Rust version by

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

After it compile successfully, you can see turto executable in directory `target` &rarr; `release`.