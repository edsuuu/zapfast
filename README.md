<p align="center">
  <img src="docs/assets/images/logo.svg" alt="ZapFast logo" width="88" height="88">
</p>

<h1 align="center">ZapFast</h1>

<p align="center"><strong>WhatsApp, native and fast.</strong><br>A lightweight WhatsApp app for Linux, macOS, and Windows.</p>

<p align="center">
  <a href="https://zapfast.rocks/download/"><strong>Download</strong></a> ·
  <a href="https://zapfast.rocks/getting-started/">Getting started</a> ·
  <a href="https://zapfast.rocks/using-zapfast/">User guide</a> ·
  <a href="CONTRIBUTING.md">Contribute</a>
</p>

<p align="center">
  <a href="Cargo.toml"><img src="https://img.shields.io/badge/Built_with-Rust-00a884" alt="Built with Rust"></a>
  <a href="https://zapfast.rocks/download/"><img src="https://img.shields.io/badge/Linux_%C2%B7_macOS_%C2%B7_Windows-24292f" alt="Linux, macOS, and Windows"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-00a884" alt="MIT license"></a>
</p>

> [!NOTE]
> **Este é o fork [edsuuu/zapfast](https://github.com/edsuuu/zapfast)** de
> [crmne/zapfast](https://github.com/crmne/zapfast). Além do upstream, ele traz:
> servidor MCP para agentes (`zapfast mcp`, veja
> [Agent access](docs/_reference/settings-and-files.md#agent-access-mcp)),
> desfoque de privacidade de fotos, nomes e mensagens, canais num lugar
> próprio, som de notificação no estilo do WhatsApp, fonte e contraste do
> WhatsApp Web atual e um balão de "digitando" do tamanho certo. Instale e
> atualize pelo apt: veja [Instalar este fork](#instalar-este-fork-debian-ubuntu-e-derivados).

ZapFast is a WhatsApp client written in Rust with
[egui](https://www.egui.rs) and [fastframe](https://fastframe.dev), the
framework behind a family of native desktop apps. It links to your phone as a
companion device through
[whatsapp-rust](https://github.com/oxidezap/whatsapp-rust) and has no browser
engine. In our Linux test it opens in under a second and uses about 200 MB of
idle RAM, against 1.13 GB for WhatsApp Web.
[See the measurements](https://zapfast.rocks/benchmarks/).

The interface is available in English, Chinese (Simplified and Traditional),
Dutch, French, German, Indonesian, Italian, Portuguese (Brazil), Russian,
Spanish, and Turkish.

<picture>
  <source media="(prefers-color-scheme: light)" srcset="docs/screenshot-light.png">
  <img src="docs/screenshot.png" alt="ZapFast showing a conversation with an attachment, voice messages, reactions, a quoted reply, and a link preview">
</picture>

<details>
<summary><strong>Watch ZapFast in action</strong></summary>

https://github.com/user-attachments/assets/2bf86b54-45fc-4add-8de7-c426c3cdad9b

</details>

[Features](#your-chats-on-your-desktop) · [Install](#install) · [Link your phone](#link-your-phone) · [Help](#guides-and-help)

## Your chats, on your desktop

| Feature | What you can do |
|---|---|
| **Messages** | Send formatted text, replies, edits, reactions, forwards, pictures, files, stickers, GIFs, polls, and voice messages. |
| **Media in the chat** | Play voice messages, videos, round video messages, GIFs, and animated stickers in place. |
| **Your history** | Keep your chats in an encrypted archive on this computer, fetch older messages from your phone as you scroll, and export locally saved chat history as text or a ZIP with downloaded attachments. |
| **Several numbers** | Link more than one WhatsApp number and switch between them in one window. |
| **Desktop controls** | Use keyboard shortcuts, select and copy text across messages, and stay linked in the tray with notifications. |
| **Themes** | Choose light, dark, system appearance, or custom colours. On Omarchy, follow your desktop theme. |
| **Privacy** | Hide locked chats behind a code, or the whole window behind an app lock. Nothing goes to a server of ours. |

## Install

### Instalar este fork (Debian, Ubuntu e derivados)

O repositório apt deste fork entrega o ZapFast com tudo o que está na nota
acima. Ele instala por cima do `.deb` oficial e, daí em diante, as atualizações
vêm deste fork:

```bash
curl -fsSL https://edsuuu.github.io/zapfast/apt/zapfast.gpg | sudo tee /usr/share/keyrings/zapfast.gpg > /dev/null
echo "deb [arch=amd64 signed-by=/usr/share/keyrings/zapfast.gpg] https://edsuuu.github.io/zapfast/apt ./" \
  | sudo tee /etc/apt/sources.list.d/zapfast.list
sudo apt update && sudo apt install zapfast
```

- **Atualizar:** `sudo apt update && sudo apt upgrade`. O ZapFast avisa quando
  sai uma versão nova [nas releases deste fork](https://github.com/edsuuu/zapfast/releases)
  e deixa a instalação para o apt.
- **Chave nova desde a 0.24.0:** o repositório passou a ser assinado com outra
  chave. Se o `apt update` reclamar de `NO_PUBKEY` ou de assinatura inválida,
  baixe a chave de novo uma vez:
  `curl -fsSL https://edsuuu.github.io/zapfast/apt/zapfast.gpg | sudo tee /usr/share/keyrings/zapfast.gpg > /dev/null`.
- **Sem o repositório:** baixe o `zapfast_<versão>_amd64.deb` da
  [última release](https://github.com/edsuuu/zapfast/releases/latest) e rode
  `sudo apt install ./zapfast_<versão>_amd64.deb`.
- **Remover:** `sudo apt remove zapfast`. Suas conversas e a sessão do
  WhatsApp ficam na sua pasta de usuário.
- **Requisitos:** Debian 12+, Ubuntu 23.04+ ou derivados (glibc 2.36), amd64.

Para publicar uma versão nova, veja [packaging/deb/README.md](packaging/deb/README.md).

### Upstream

As opções abaixo instalam o ZapFast do upstream (crmne/zapfast), sem os
recursos deste fork.

| Platform | Installation |
|---|---|
| **macOS** | `brew install --cask crmne/tap/zapfast`, or [download the Mac app](https://zapfast.rocks/download/#macos). |
| **Arch Linux** | `yay -S zapfast-bin` |
| **Windows** | Choose your build on the [Download page](https://zapfast.rocks/download/#windows). |
| **Other Linux** | Find Nix, AppImage, and other options on the [Download page](https://zapfast.rocks/download/#linux). |
| **From source** | Follow [Getting started](https://zapfast.rocks/getting-started/) for dependencies and commands. |

## Link your phone

1. Open ZapFast. It shows a QR code.
2. On your phone, open WhatsApp, then **Settings**, **Linked devices**,
   **Link a device**, and scan the code. Or click **Link with phone number**
   and enter the eight-character code on your phone.
3. Your recent chats arrive from the phone. Older messages follow as you
   scroll up.

The link survives restarts, and your phone does not need to stay online to
read what ZapFast has already stored. See
[Getting started](https://zapfast.rocks/getting-started/) for the full
walkthrough and [How it links](https://zapfast.rocks/how-it-links/) for the
details.

## Guides and help

The complete guide lives at **[zapfast.rocks](https://zapfast.rocks/)**.

| You want to… | Read |
|---|---|
| Know what ZapFast does and does not do yet | [What is ZapFast?](https://zapfast.rocks/what-is-zapfast/) |
| Link your phone or build from source | [Getting started](https://zapfast.rocks/getting-started/) |
| Learn messages, stickers, polls, and shortcuts | [Using ZapFast](https://zapfast.rocks/using-zapfast/) |
| Find settings, stored files, encryption, the app lock, and updates | [Settings and files](https://zapfast.rocks/settings-and-files/) |
| Make your own theme | [Making a theme](https://zapfast.rocks/themes/) |
| Understand what stays on this computer | [How it links](https://zapfast.rocks/how-it-links/) |
| Understand account risk | [Account safety](https://zapfast.rocks/what-is-zapfast/#account-safety) |
| Report a bug or propose a feature | Read [Contributing](CONTRIBUTING.md), then use the [issue forms](https://github.com/crmne/zapfast/issues/new/choose). |

## Contributing

Read [CONTRIBUTING.md](CONTRIBUTING.md) before opening an issue or pull
request. To look at the interface without a WhatsApp account, run:

```sh
cargo run --features demo -- --demo
```

[AGENTS.md](AGENTS.md) describes the architecture, [DEMO.md](DEMO.md) the
demo pages, screenshots, and recorded tours, and
[PACKAGING.md](PACKAGING.md) releases and packages.

Native notifications use Fastframe on Linux, macOS and Windows. ZapFast keeps
its account/chat routing, privacy settings, notification limits and sound
previews. Clicks open the announced message; reading a chat withdraws its
tracked notifications. OS permissions and notification presentation still apply.


## More native apps

**Want Spotify just as fast and native?** [Spotifast](https://spotifast.rocks)
is ZapFast's sibling. Both are built on
[fastframe](https://fastframe.dev).

## Acknowledgements

ZapFast uses [whatsapp-rust](https://github.com/oxidezap/whatsapp-rust),
[egui](https://www.egui.rs), [fastframe](https://fastframe.dev), the
[Inter](https://rsms.me/inter/) typeface and Noto Color Emoji (OFL), and
[Lucide](https://lucide.dev) icons (ISC), also used for the chat wallpaper
doodles. The notification sounds are [Pidgin](https://pidgin.im)'s, under the
GPL-2.0 (see `assets/sounds/`).

ZapFast is an unofficial client and is not affiliated with WhatsApp or Meta.
Using an unofficial client may be against WhatsApp's terms of service and
could get an account suspended. Use it at your own risk. WhatsApp is a
trademark of WhatsApp LLC.

Licensed under the [MIT License](LICENSE).

## Star history

<a href="https://www.star-history.com/?repos=crmne%2Fzapfast&amp;type=date">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/chart?repos=crmne/zapfast&amp;type=date&amp;theme=dark">
    <img alt="ZapFast GitHub stars over time" src="https://api.star-history.com/chart?repos=crmne/zapfast&amp;type=date">
  </picture>
</a>
