<script setup lang="ts">
defineProps<{
    title: string
}>()
</script>

<template>
    <v-card class="mb-4">
        <v-card-title class="text-h6 pb-0">
            {{ title }}
        </v-card-title>
        <v-card-text>
            <slot />
        </v-card-text>
    </v-card>
</template>

<style scoped>
.v-card {
    border-radius: 12px;
}
</style>
