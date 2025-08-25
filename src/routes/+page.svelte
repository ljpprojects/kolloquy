<nav>
    <section class="user-icon">
        <svg viewBox="0 0 300 300">
            <defs>
                <linearGradient id="fg1" x1="0%" y1="0%" x2="100%" y2="0%">
                    <stop offset="0%" stop-color="hsl(220.5, 100%, 45%)" />
                    <stop offset="100%" stop-color="hsl(240.5, 100%, 45%)" />
                </linearGradient>
                <linearGradient id="fg2" x1="0%" y1="0%" x2="100%" y2="0%">
                    <stop offset="0%" stop-color="hsl(40.5, 100%, 45%)" />
                    <stop offset="100%" stop-color="hsl(120.5, 100%, 45%)" />
                </linearGradient>
                <linearGradient id="fg3" x1="0%" y1="0%" x2="100%" y2="0%">
                    <stop offset="0%" stop-color="hsl(190.5, 100%, 50%)" />
                    <stop offset="100%" stop-color="hsl(110.5, 100%, 50%)" />
                </linearGradient>
                <linearGradient id="bg" x1="0%" y1="0%" x2="100%" y2="0%">
                    <stop offset="0%" stop-color="hsl(171.5, 100%, 45%)" />
                    <stop offset="100%" stop-color="hsl(184, 100%, 45%)" />
                </linearGradient>
                <filter id="noise">
                    <feTurbulence
                        baseFrequency="5"
                        numOctaves="3"
                        stitchTiles="noStitch"
                        type="fractalNoise"
                    />
                </filter>
                <mask id="outsideMask">
                    <rect width="300" height="300" fill="white" />
                    <circle cx="150" cy="137" r="93.75" fill="black" />
                </mask>
            </defs>
            <rect width="300" height="300" fill="url(#bg)" />
            <circle cx="150" cy="0" r="93.75" fill="url(#fg1)" />
            <circle cx="150" cy="300" r="93.75" fill="url(#fg1)" />
            <rect
                width="300"
                height="300"
                filter="url(#noise)"
                opacity="0.9"
                mask="url(#outsideMask)"
            />
            <circle
                cx="150"
                cy="119"
                r="93.75"
                fill="url(#fg2)"
                style="mix-blend-mode: multiply;"
            />
            <circle
                cx="150"
                cy="120"
                r="93.75"
                fill="url(#fg2)"
                style="mix-blend-mode: difference;"
            />
            <circle
                cx="150"
                cy="131"
                r="93.75"
                fill="url(#fg1)"
                style="mix-blend-mode: exclusion;"
            />
            <circle
                cx="150"
                cy="127"
                r="93.75"
                fill="url(#fg2)"
                style="mix-blend-mode: color;"
            />
            <circle
                cx="150"
                cy="183"
                r="93.75"
                fill="url(#fg3)"
                style="mix-blend-mode: difference;"
            />
            <circle
                cx="150"
                cy="148"
                r="93.75"
                fill="url(#fg1)"
                style="mix-blend-mode: hue;"
            />
        </svg>
    </section>
</nav>

<style lang="scss">
    .user-icon > svg {
        width: 20vmin;
        height: 20vmin;
    }
</style>
