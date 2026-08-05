# Passvault

An offline, encrypted password vault for your desktop.

## Personal Notes

This is an application that stores your passwords in an encrypted vault on your own device. A lot of people (including me) have a bad habit of forgetting passwords and jotting them down in notes apps as plain text, which is pretty ironic for programmers like myself.

I made Passvault to safely store passwords, PINs, API keys, and other important credentials, and I hope people find it useful. Right now it's only available for Windows, but versions for macOS, Linux, and Android are on the way.

## How it works (Da Baby Language)

Think of Passvault as a diary with a really smart lock. Everything you save goes into one file on your computer, but that file is scrambled beyond recognition unless you know your PIN.



- **AES-256-GCM** is the lock itself. AES is the scrambling method used by banks, governments, and pretty much anyone who takes security seriously. The "256" refers to the size of the key: there are roughly 1 followed by 77 zeros possible keys. Guessing the right one by accident isn't going to happen in a human lifetime, or a million of them. The "GCM" part means the lock also detects if someone has tampered with your file.

- **Argon2id** is the key smith. When you type your PIN, Passvault doesn't use the PIN directly as the key (that would be too weak). Instead it feeds the PIN through Argon2id, which is deliberately slow and memory-hungry. It uses 128 MB of RAM and takes a fraction of a second on your device. That fraction of a second sounds tiny, but if an attacker steals your vault file and tries to guess your PIN by brute force, they hit that same delay for *every* guess. Argon2id is the winner of the international Password Hashing Competition and is currently the gold standard.

- **Keyfile (optional 2FA)** is a second layer of security. You pick any file you already own, such as a photo or a document, and Passvault mixes it into the key. From then on, unlocking requires **both** your PIN and that exact file. It's the "something you know + something you have" pattern, like a bank card plus a PIN.

- **Zeroize**: when you lock the vault, the decryption key is wiped from your computer's memory. Not just "deleted" (which often leaves the data lying around), but actively overwritten with zeros. So even if someone freezes your RAM and tries to read it, the key isn't there.

- **Compression before encryption**: before your vault is encrypted, it's compressed with `deflate`. This makes the file smaller, which is what lets your entire vault fit in a single QR code for backup.

### Nothing leaves your machine

There is no server. No cloud account. No telemetry. No "sync" that quietly uploads your vault somewhere. Passvault lives entirely on your disk. If you want a backup or to move to another device, Passvault generates a QR code (or a `.passbackup` file) and you decide when and how to move it.

## Features

- AES-256-GCM encryption with Argon2id key derivation
- Optional keyfile 2FA
- System tray + configurable global hotkey (default `Ctrl+Shift+P`) to summon the window
- Auto-lock after 5 minutes of idle time
- Clipboard auto-clears 20 seconds after you copy a password
- Exponential lockout after wrong PIN attempts (15s, 30s, 60s, 5m, 15m)
- QR code and `.passbackup` file export / import for cross-device backup
- Fully offline, no network required, ever

## Install

Grab the latest installer from the [Releases](../../releases) page.

## Build from source

```bash
git clone https://github.com/Ravandratriyanto/Passvault.git
cd Passvault
npm install
npm run tauri dev      # run in dev mode
npm run tauri build    # produce a production installer
```

Requires Node 18+, Rust (stable toolchain), and, on Linux, the WebKitGTK dependencies listed in the [Tauri prerequisites](https://tauri.app/start/prerequisites/).

## Platforms

- READY Windows
- NOTREADY macOS
- NOTREADY Linux
- NOTREADY Android

ON DA WAYYYYYYY.

## License

MIT. See [LICENSE](LICENSE). Free to use, copy, modify, and distribute.
