<script lang="ts">
    import favicon from "$lib/assets/favicon.svg";

    let { children } = $props();
</script>

<svelte:head>
    <link rel="icon" href={favicon} />
</svelte:head>

{@render children?.()}

<style lang="scss">
    $bg-light: #fafafa;
    $bg-dark: #1e1e1e;

    :global(body) {
        background-color: $bg-light;
    }

    @media (prefers-color-scheme: dark) {
        :global(body) {
            background-color: $bg-dark;
        }
    }
</style>
