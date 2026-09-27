import { defineConfig } from 'vite'
import { viteSingleFile } from 'vite-plugin-singlefile'

export default defineConfig({
    plugins: [viteSingleFile()],

    build: {
        target: 'esnext',
        minify: 'terser',
        cssCodeSplit: false,
        assetsInlineLimit: Infinity,

        terserOptions: {
            compress: {
                passes: 3,
            },
            mangle: true,
            format: {
                comments: false,
            },
        },

        rollupOptions: {
            output: {
                inlineDynamicImports: true,
            },
        },
    },
})
