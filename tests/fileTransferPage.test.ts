import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const page = readFileSync("src/pages/FileTransfer.vue", "utf8");
const router = readFileSync("src/router/index.ts", "utf8");
const sidebar = readFileSync("src/components/layout/Sidebar.vue", "utf8");
const app = readFileSync("src/App.vue", "utf8");
const shell = readFileSync("src/components/layout/AppShell.vue", "utf8");
const store = readFileSync("src/stores/fileTransfer.ts", "utf8");
const backend = readFileSync("src-tauri/src/file_transfer.rs", "utf8");
const handleFileOffer = backend.match(
  /pub async fn handle_file_offer[\s\S]*?\npub async fn handle_file_accept/,
)?.[0] ?? "";

assert.match(router, /path:\s*"\/files"/);
assert.match(sidebar, /文件传输|鏂囦欢浼犺緭/);
assert.match(page, /发送文件/);
assert.match(page, /传输任务/);
assert.doesNotMatch(page, /WebSocket 鍙紶鎺у埗娑堟伅/);
assert.match(page, /:disabled="sendDisabled"/);
assert.match(page, /import RefreshButton from "@\/components\/ui\/RefreshButton\.vue"/);
assert.match(page, /:refresh="\(\) => fileTransferStore\.refresh\(\)"/);
assert.match(page, /:failed="\(\) => Boolean\(fileTransferStore\.error\)"/);
assert.match(app, /useFileTransferStore/);
assert.match(shell, /FileTransferOfferDialog/);
assert.doesNotMatch(store, /"file-transfer-offer"[\s\S]*this\.acceptOffer\(task\.transferId\)/);
assert.match(handleFileOffer, /manager\.handle_offer\(app, offer\)/);
assert.doesNotMatch(handleFileOffer, /accept_receive/);

assert.match(page, /selectedFiles/);
assert.match(page, /selectFiles/);
assert.match(page, /sendFiles/);
assert.match(page, /files\.length/);
assert.match(page, /currentTransferFileName/);
assert.match(page, /selectedFilesTotalSize/);
assert.match(page, /fileProgressPercent/);
