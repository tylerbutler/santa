import expressiveCode from "astro-expressive-code";
import { defineConfig } from "astro/config";

export default defineConfig({
  integrations: [
    expressiveCode({
      themes: ["github-light"],
      useThemedScrollbars: false,
      styleOverrides: {
        borderRadius: "0",
        borderWidth: "0",
        codeBackground: "transparent",
        codeForeground: "#171b19",
        codeFontFamily: '"JetBrains Mono Variable", ui-monospace, monospace',
        codeFontSize: "inherit",
        codeFontWeight: "400",
        codeLineHeight: "1.72",
        codePaddingBlock: "0",
        codePaddingInline: "0",
      },
    }),
  ],
  output: "static",
});
