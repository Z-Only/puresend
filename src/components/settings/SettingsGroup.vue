<script setup lang="ts">
defineProps<{
    title: string
}>()
</script>

<template>
    <v-card class="mb-4">
        <v-card-title class="text-h6 pb-2">
            {{ title }}
        </v-card-title>
        <v-card-text class="pt-2">
            <slot />
        </v-card-text>
    </v-card>
</template>

<style scoped>
.v-card {
    border-radius: 12px;
}
</style>
