<script setup lang="ts">
import { computed } from "vue";

import { getConnectionBadgeView } from "@/lib/statusBadge";
import type { SyncState } from "@/types/status";

const props = defineProps<{
  state: SyncState;
  label: string;
}>();

const badgeView = computed(() => getConnectionBadgeView(props.state));
</script>

<template>
  <span
    class="inline-flex h-8 items-center gap-2 rounded-lg border px-3 text-sm font-semibold transition-colors"
    :class="badgeView.containerClass"
  >
    <span
      class="h-2 w-2 rounded-full"
      :class="badgeView.dotClass"
    />
    {{ label }}
  </span>
</template>
