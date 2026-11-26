# Alogian Code

A terminal coding assistant connected to your Alogian account. Work with local files, review commands before running them, plan changes, and resume saved conversations.

## Install

Requires Windows x64 or macOS (Apple Silicon or Intel), and Node.js 20 or newer. Rust is not required to install or use the published package. The package includes native builds for supported platforms; installation does not compile source code on your computer.

```sh
npm install --global lume-code
alogian
```

Start from your project folder, or select one explicitly:

```sh
alogian --project "C:\path\to\project"
```

Sign in through your browser with your Alogian account. If the browser does not open automatically, copy the URL shown in the terminal. Your account credential is stored in Windows Credential Manager or macOS Keychain.

```powershell
alogian login
alogian logout
alogian --help
alogian --version
```

Type `/` inside Alogian for commands, `?` for keyboard shortcuts, or use Shift-Tab to switch between Plan and Edit. Local commands require your approval and run with your user permissions.

Alogian Code uses the public Alogian API and shares your account's usage allowance. A network connection and a Alogian account are required. No backend installation is needed.

Update with `npm install --global lume-code@latest`. Remove with `npm uninstall --global lume-code`.
