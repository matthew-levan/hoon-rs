import * as path from "path";
import * as fs from "fs";
import * as os from "os";
import * as vscode from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;

/**
 * Returns the path to the bundled hoon-lsp binary for the current platform,
 * or throws if the platform is unsupported.
 */
function getBinaryPath(context: vscode.ExtensionContext): string {
  const platform = os.platform();
  const arch = os.arch();

  let binaryName: string;
  if (platform === "linux" && arch === "x64") {
    binaryName = "hoon-lsp-linux-x86_64";
  } else if (platform === "darwin" && arch === "arm64") {
    binaryName = "hoon-lsp-darwin-aarch64";
  } else if (platform === "darwin" && arch === "x64") {
    binaryName = "hoon-lsp-darwin-x86_64";
  } else if (platform === "win32" && arch === "x64") {
    binaryName = "hoon-lsp-windows-x86_64.exe";
  } else {
    throw new Error(
      `hoon-lsp: unsupported platform ${platform}/${arch}. ` +
        `Please build hoon-lsp from source and add it to your PATH.`
    );
  }

  return context.asAbsolutePath(path.join("bin", binaryName));
}

/**
 * Locate the hoon-lsp binary. Tries the bundled binary first, then falls
 * back to `hoon-lsp` on the user's PATH.
 */
function resolveServerBinary(
  context: vscode.ExtensionContext
): string | undefined {
  try {
    const bundled = getBinaryPath(context);
    if (fs.existsSync(bundled)) {
      // Ensure the binary is executable on Unix
      if (os.platform() !== "win32") {
        fs.chmodSync(bundled, 0o755);
      }
      return bundled;
    }
  } catch {
    // Platform unsupported — fall through to PATH lookup
  }

  // Try PATH as a fallback
  const pathDirs = (process.env.PATH ?? "").split(path.delimiter);
  for (const dir of pathDirs) {
    const candidate = path.join(dir, os.platform() === "win32" ? "hoon-lsp.exe" : "hoon-lsp");
    if (fs.existsSync(candidate)) {
      return candidate;
    }
  }

  return undefined;
}

export function activate(context: vscode.ExtensionContext): void {
  const serverBinary = resolveServerBinary(context);

  if (!serverBinary) {
    vscode.window
      .showErrorMessage(
        "hoon-lsp binary not found. Install it with `cargo install hoon-lsp` or download a release.",
        "Open Releases"
      )
      .then((choice) => {
        if (choice === "Open Releases") {
          vscode.env.openExternal(
            vscode.Uri.parse(
              "https://github.com/MatthewLeVan/hoon-rs/releases"
            )
          );
        }
      });
    // Still activate without LSP so syntax highlighting works
    return;
  }

  const serverOptions: ServerOptions = {
    command: serverBinary,
    args: ["--stdio"],
    transport: TransportKind.stdio,
  };

  const config = vscode.workspace.getConfiguration("hoonLsp");

  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "hoon" }],
    synchronize: {
      fileEvents: vscode.workspace.createFileSystemWatcher("**/*.hoon"),
    },
    initializationOptions: {
      debounceMs: config.get<number>("debounceMs", 75),
      workspaceRefreshMs: config.get<number>("workspaceRefreshMs", 30000),
      maxWorkspaceFiles: config.get<number>("maxWorkspaceFiles", 10000),
      followSymlinks: config.get<boolean>("followSymlinks", true),
    },
    outputChannelName: "Hoon Language Server",
    traceOutputChannel: vscode.window.createOutputChannel(
      "Hoon Language Server (trace)"
    ),
  };

  client = new LanguageClient(
    "hoon",
    "Hoon Language Server",
    serverOptions,
    clientOptions
  );

  // Re-send configuration when user changes settings
  context.subscriptions.push(
    vscode.workspace.onDidChangeConfiguration((e) => {
      if (e.affectsConfiguration("hoonLsp")) {
        client?.sendNotification("workspace/didChangeConfiguration", {
          settings: vscode.workspace.getConfiguration("hoonLsp"),
        });
      }
    })
  );

  context.subscriptions.push(client);
  client.start();
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }
  return client.stop();
}
