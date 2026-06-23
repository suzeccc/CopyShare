import packageJson from "../../package.json" with { type: "json" };

export const APP_VERSION = packageJson.version;
export const GITHUB_REPOSITORY_URL = "https://github.com/suzeccc/Copy-share";
export const AUTHOR_NAME = "suzecc";
export const UPDATE_URL = `${GITHUB_REPOSITORY_URL}/releases/latest`;
export const RELEASE_API_URL =
  "https://api.github.com/repos/suzeccc/Copy-share/releases/latest";

export interface ReleaseInfo {
  version: string;
  url: string;
}

export interface UpdateState {
  hasUpdate: boolean;
  latestVersion: string;
  updateUrl: string;
}

export function normalizeVersion(version: string): string {
  const match = version.trim().match(/\d+(?:\.\d+){0,2}/);
  return match?.[0] ?? version.trim().replace(/^v/i, "");
}

export function getUpdateState(
  currentVersion: string,
  latestRelease: ReleaseInfo,
): UpdateState {
  const current = normalizeVersion(currentVersion);
  const latest = normalizeVersion(latestRelease.version);

  return {
    hasUpdate: compareVersions(latest, current) > 0,
    latestVersion: latest,
    updateUrl: latestRelease.url,
  };
}

export async function getLatestRelease(): Promise<ReleaseInfo> {
  const response = await fetch(RELEASE_API_URL, {
    headers: {
      Accept: "application/vnd.github+json",
    },
  });

  if (!response.ok) {
    throw new Error("无法获取最新版本信息");
  }

  const release = await response.json() as {
    tag_name?: string;
    name?: string;
    html_url?: string;
  };
  const version = release.tag_name || release.name;

  if (!version) {
    throw new Error("最新版本信息缺少版本号");
  }

  return {
    version,
    url: release.html_url || UPDATE_URL,
  };
}

function compareVersions(left: string, right: string): number {
  const leftParts = versionParts(left);
  const rightParts = versionParts(right);
  const maxLength = Math.max(leftParts.length, rightParts.length);

  for (let index = 0; index < maxLength; index += 1) {
    const diff = (leftParts[index] ?? 0) - (rightParts[index] ?? 0);
    if (diff !== 0) {
      return diff;
    }
  }

  return 0;
}

function versionParts(version: string): number[] {
  return normalizeVersion(version)
    .split(".")
    .map((part) => Number(part))
    .filter((part) => Number.isFinite(part));
}
