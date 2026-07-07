<script setup lang="ts">
import { computed, onMounted } from "vue";
import { FolderOpen, Send, X } from "lucide-vue-next";

import Button from "@/components/ui/Button.vue";
import Card from "@/components/ui/Card.vue";
import RefreshButton from "@/components/ui/RefreshButton.vue";
import {
  currentTransferFileName,
  fileProgressPercent,
  fileTransferStatusLabel,
  formatTransferSize,
  selectedFilesTotalSize,
  transferProgressPercent,
} from "@/lib/fileTransfer";
import { useDevicesStore } from "@/stores/devices";
import { useFileTransferStore } from "@/stores/fileTransfer";

const devicesStore = useDevicesStore();
const fileTransferStore = useFileTransferStore();
const sendDisabled = computed(() => fileTransferStore.sendDisabled);
const selectedFiles = computed(() => fileTransferStore.selectedFiles);
const selectedTotalSize = computed(() => selectedFilesTotalSize(selectedFiles.value));
const targetDevices = computed(() => devicesStore.connected);

onMounted(() => {
  if (!devicesStore.devices.length) {
    void devicesStore.refresh();
  }
  void fileTransferStore.refresh();
});
</script>

<template>
  <div class="grid gap-5">
    <section class="grid gap-5">
      <Card>
        <div>
          <p class="text-sm font-semibold text-white">发送文件</p>
          <p class="mt-2 text-sm leading-6 text-[color:var(--muted-text)]">
            支持向已互相信任的局域网 PC 发送多个普通文件。WebSocket 只发送控制消息，文件内容通过局域网 HTTP 流式传输。
          </p>
        </div>

        <div class="mt-5 grid gap-4">
          <div class="rounded-lg border border-[color:var(--main-line-soft)] bg-[color:var(--panel-bg-soft)] p-4">
            <div class="flex flex-wrap items-start justify-between gap-3">
              <div class="min-w-0">
                <p class="text-xs font-medium uppercase tracking-wide text-slate-500">
                  已选文件
                </p>
                <p class="mt-1 text-sm font-semibold text-white">
                  <span v-if="selectedFiles.length">
                    {{ selectedFiles.length }} 个文件 · {{ formatTransferSize(selectedTotalSize) }}
                  </span>
                  <span v-else>尚未选择文件</span>
                </p>
                <ul
                  v-if="selectedFiles.length"
                  class="mt-2 grid max-h-28 gap-1 overflow-auto text-xs text-slate-400"
                >
                  <li
                    v-for="file in selectedFiles.slice(0, 5)"
                    :key="file.path"
                    class="truncate"
                  >
                    {{ file.name }} · {{ formatTransferSize(file.size) }}
                  </li>
                  <li v-if="selectedFiles.length > 5">
                    还有 {{ selectedFiles.length - 5 }} 个文件
                  </li>
                </ul>
              </div>
              <Button variant="secondary" @click="fileTransferStore.selectFiles">
                选择文件
              </Button>
            </div>
          </div>

          <label class="grid gap-2">
            <span class="text-xs font-medium uppercase tracking-wide text-slate-500">
              目标设备
            </span>
            <select
              class="h-10 rounded-md border border-[color:var(--main-line-soft)] bg-[color:var(--main-bg-soft)] px-3 text-sm text-white outline-none transition focus:border-[color:var(--accent-line)]"
              v-model="fileTransferStore.targetDeviceId"
            >
              <option value="">选择已信任设备</option>
              <option
                v-for="device in targetDevices"
                :key="device.id"
                :value="device.id"
              >
                {{ device.name }} · {{ device.ip }}:{{ device.port }}
              </option>
            </select>
          </label>

          <p
            v-if="fileTransferStore.error"
            class="rounded-md border border-red-500/40 bg-red-500/10 px-3 py-2 text-sm text-red-100"
          >
            {{ fileTransferStore.error }}
          </p>

          <Button
            variant="primary"
            :disabled="sendDisabled"
            @click="fileTransferStore.sendFiles"
          >
            <Send class="h-4 w-4" />
            发送
          </Button>
        </div>
      </Card>
    </section>

    <Card>
      <div class="flex flex-wrap items-start justify-between gap-4">
        <div>
          <p class="text-sm font-semibold text-white">传输任务</p>
          <p class="mt-2 text-sm text-[color:var(--muted-text)]">
            显示发送和接收任务的总进度、当前文件、状态和结果。
          </p>
        </div>
        <RefreshButton
          :refresh="() => fileTransferStore.refresh()"
          :failed="() => Boolean(fileTransferStore.error)"
          variant="secondary"
          size="md"
        />
      </div>

      <div v-if="fileTransferStore.tasks.length" class="mt-5 grid gap-3">
        <article
          v-for="task in fileTransferStore.tasks"
          :key="task.transferId"
          class="rounded-lg border border-[color:var(--main-line-soft)] bg-[color:var(--panel-bg-soft)] p-4"
        >
          <div class="flex flex-wrap items-start justify-between gap-3">
            <div class="min-w-0">
              <p class="truncate text-sm font-semibold text-white">
                {{ task.files.length === 1 ? task.files[0]?.name : `${task.files.length} 个文件` }}
              </p>
              <p class="mt-1 text-xs text-slate-400">
                {{ task.direction === "send" ? "发送到" : "接收自" }}
                {{ task.peerDeviceName }} · {{ formatTransferSize(task.totalSize) }}
              </p>
              <p
                v-if="['accepted', 'transferring'].includes(task.status)"
                class="mt-1 text-xs text-slate-400"
              >
                当前文件：{{ currentTransferFileName(task) }}
              </p>
            </div>
            <span class="rounded-full border border-[color:var(--main-line-soft)] px-2.5 py-1 text-xs text-slate-300">
              {{ fileTransferStatusLabel(task.status) }}
            </span>
          </div>

          <div class="mt-4">
            <div class="h-2 overflow-hidden rounded-full bg-[color:var(--main-bg-muted)]">
              <div
                class="h-full rounded-full bg-[color:var(--accent-text)] transition-all"
                :style="{ width: `${transferProgressPercent(task)}%` }"
              ></div>
            </div>
            <div class="mt-2 flex justify-between text-xs text-slate-400">
              <span>
                {{ formatTransferSize(task.transferredBytes) }} / {{ formatTransferSize(task.totalSize) }}
              </span>
              <span>{{ transferProgressPercent(task) }}%</span>
            </div>
          </div>

          <details v-if="task.files.length > 1" class="mt-4">
            <summary class="cursor-pointer text-xs text-slate-400">
              查看 {{ task.files.length }} 个文件
            </summary>
            <div class="mt-3 grid gap-2">
              <div
                v-for="file in task.files"
                :key="file.id"
                class="rounded-md border border-[color:var(--main-line-soft)] bg-[color:var(--main-bg-soft)] px-3 py-2"
              >
                <div class="flex items-center justify-between gap-3 text-xs">
                  <span class="min-w-0 truncate text-slate-200">{{ file.name }}</span>
                  <span class="shrink-0 text-slate-500">
                    {{ file.status }} · {{ fileProgressPercent(file) }}%
                  </span>
                </div>
                <div class="mt-2 h-1.5 overflow-hidden rounded-full bg-[color:var(--main-bg-muted)]">
                  <div
                    class="h-full rounded-full bg-[color:var(--accent-text)]"
                    :style="{ width: `${fileProgressPercent(file)}%` }"
                  ></div>
                </div>
              </div>
            </div>
          </details>

          <p v-if="task.error" class="mt-3 text-xs text-red-200">
            {{ task.error }}
          </p>

          <div class="mt-4 flex flex-wrap justify-end gap-2">
            <Button
              v-if="['pending', 'accepted', 'transferring'].includes(task.status)"
              size="sm"
              variant="danger"
              @click="fileTransferStore.cancel(task.transferId)"
            >
              <X class="h-4 w-4" />
              取消
            </Button>
            <Button
              v-if="task.status === 'completed'"
              size="sm"
              variant="secondary"
              @click="fileTransferStore.openFolder"
            >
              <FolderOpen class="h-4 w-4" />
              打开文件夹
            </Button>
          </div>
        </article>
      </div>
      <div
        v-else
        class="mt-5 rounded-lg border border-dashed border-[color:var(--main-line-soft)] px-4 py-12 text-center text-sm text-[color:var(--subtle-text)]"
      >
        暂无文件传输任务。
      </div>
    </Card>
  </div>
</template>
