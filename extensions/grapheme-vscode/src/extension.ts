import * as fs from "node:fs/promises";
import { createWriteStream } from "node:fs";
import * as path from "node:path";
import * as https from "node:https";

import * as vscode from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  const binaryPath = await resolveLspBinary(context);

  const serverOptions: ServerOptions = {
    command: binaryPath,
    args: [],
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "grapheme" }],
    synchronize: {
      fileEvents: vscode.workspace.createFileSystemWatcher("**/*.gr"),
    },
  };

  client = new LanguageClient(
    "graphemeLsp",
    "Grapheme Language Server",
    serverOptions,
    clientOptions,
  );

  await client.start();
}

export async function deactivate(): Promise<void> {
  if (client) {
    await client.stop();
    client = undefined;
  }
}

async function resolveLspBinary(context: vscode.ExtensionContext): Promise<string> {
  const cfg = vscode.workspace.getConfiguration("grapheme.lsp");
  const explicitPath = cfg.get<string>("binaryPath", "").trim();
  if (explicitPath.length > 0) {
    await assertExecutable(explicitPath);
    return explicitPath;
  }

  const bundled = bundledBinaryPath(context);
  if (bundled && (await fileExists(bundled))) {
    await makeExecutableIfNeeded(bundled);
    return bundled;
  }

  const autoDownload = cfg.get<boolean>("autoDownload", true);
  if (!autoDownload) {
    throw new Error("grapheme-lsp binary not found and autoDownload=false");
  }

  const repo = cfg.get<string>("releaseRepo", "entasislabs/grapheme").trim();
  const tag = cfg.get<string>("releaseTag", "latest").trim() || "latest";

  const localPath = await downloadLspFromRelease(context, repo, tag);
  await makeExecutableIfNeeded(localPath);
  return localPath;
}

function bundledBinaryPath(context: vscode.ExtensionContext): string | undefined {
  const ext = process.platform === "win32" ? ".exe" : "";
  const candidate = path.join(context.extensionPath, "server", `grapheme-lsp${ext}`);
  return candidate;
}

async function downloadLspFromRelease(
  context: vscode.ExtensionContext,
  repo: string,
  tag: string,
): Promise<string> {
  const metadataUrl =
    tag === "latest"
      ? `https://api.github.com/repos/${repo}/releases/latest`
      : `https://api.github.com/repos/${repo}/releases/tags/${encodeURIComponent(tag)}`;

  const release = (await getJson(metadataUrl)) as {
    tag_name: string;
    assets: Array<{ name: string; browser_download_url: string }>;
  };

  const target = platformTarget();
  const ext = process.platform === "win32" ? ".exe" : "";
  const candidates = [
    `grapheme-lsp-${target}${ext}`,
    `grapheme-lsp-${target}`,
    `grapheme-lsp${ext}`,
  ];

  const asset = release.assets.find((a) => candidates.includes(a.name));
  if (!asset) {
    throw new Error(
      `No matching grapheme-lsp asset for ${target} in release ${release.tag_name}. Expected one of: ${candidates.join(", ")}`,
    );
  }

  const storageDir = path.join(
    context.globalStorageUri.fsPath,
    "lsp",
    release.tag_name,
  );
  await fs.mkdir(storageDir, { recursive: true });

  const localPath = path.join(storageDir, `grapheme-lsp${ext}`);
  if (!(await fileExists(localPath))) {
    await downloadFile(asset.browser_download_url, localPath);
  }

  return localPath;
}

function platformTarget(): string {
  const platform = process.platform;
  const arch = process.arch;

  const normPlatform =
    platform === "win32" ? "windows" :
    platform === "darwin" ? "macos" :
    platform === "linux" ? "linux" :
    platform;

  const normArch =
    arch === "x64" ? "x64" :
    arch === "arm64" ? "arm64" :
    arch;

  return `${normPlatform}-${normArch}`;
}

function getJson(url: string): Promise<unknown> {
  return new Promise((resolve, reject) => {
    const req = https.get(
      url,
      {
        headers: {
          "User-Agent": "grapheme-vscode",
          Accept: "application/vnd.github+json",
        },
      },
      (res) => {
        if (!res.statusCode || res.statusCode < 200 || res.statusCode >= 300) {
          reject(new Error(`GET ${url} failed with ${res.statusCode}`));
          res.resume();
          return;
        }

        const chunks: Buffer[] = [];
        res.on("data", (d) => chunks.push(Buffer.isBuffer(d) ? d : Buffer.from(d)));
        res.on("end", () => {
          try {
            const parsed = JSON.parse(Buffer.concat(chunks).toString("utf8"));
            resolve(parsed);
          } catch (err) {
            reject(err);
          }
        });
      },
    );

    req.on("error", reject);
  });
}

function downloadFile(url: string, outPath: string): Promise<void> {
  return new Promise((resolve, reject) => {
    const req = https.get(
      url,
      {
        headers: {
          "User-Agent": "grapheme-vscode",
          Accept: "application/octet-stream",
        },
      },
      (res) => {
        if (res.statusCode && res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
          res.resume();
          downloadFile(res.headers.location, outPath).then(resolve).catch(reject);
          return;
        }

        if (!res.statusCode || res.statusCode < 200 || res.statusCode >= 300) {
          reject(new Error(`Download failed: ${url} (${res.statusCode})`));
          res.resume();
          return;
        }

        const file = createWriteStream(outPath, { mode: 0o755 });
        res.pipe(file);
        file.on("finish", () => {
          file.close();
          resolve();
        });
        file.on("error", (err) => {
          reject(err);
        });
      },
    );

    req.on("error", reject);
  });
}

async function fileExists(filePath: string): Promise<boolean> {
  try {
    await fs.access(filePath);
    return true;
  } catch {
    return false;
  }
}

async function assertExecutable(filePath: string): Promise<void> {
  if (!(await fileExists(filePath))) {
    throw new Error(`LSP binary not found at ${filePath}`);
  }
  await makeExecutableIfNeeded(filePath);
}

async function makeExecutableIfNeeded(filePath: string): Promise<void> {
  if (process.platform !== "win32") {
    await fs.chmod(filePath, 0o755);
  }
}
