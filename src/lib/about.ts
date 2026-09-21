import packageJson from "../../package.json" with { type: "json" };

export const APP_VERSION = packageJson.version;
export const GITHUB_REPOSITORY_URL = "https://github.com/suzeccc/CopyShare";
export const AUTHOR_NAME = "suzecc";
export const UPDATE_URL = `${GITHUB_REPOSITORY_URL}/releases/latest`;
