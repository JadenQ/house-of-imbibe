// web/eslint.config.js — ESLint flat config（ESLint v9+）。
//
// 唯一职责：守卫前端分层禁令 —— net/、game-state/、protocol/ 三个目录
// 禁止 import 'phaser'（docs/development-plan.md §2.3；CLAUDE.md「其他硬约束」；
// 与 web/src/protocol/types.ts 顶部注释一致）。scene/ 与 src/main.ts 是允许使用
// phaser 的层，故不在守卫范围内。
//
// 设计取舍：
// - .ts 文件需 @typescript-eslint/parser 解析（ESLint 内置 espree 不认 TS 语法）。
// - 故意不启用 js.configs.recommended 或 @typescript-eslint recommended 全套规则，
//   以免对既有代码产生非预期 lint 报错；本配置只跑 no-restricted-imports 这一条守卫。
// - files 只圈定三个被守卫目录，其余文件（含 scene/、main.ts）不被 lint，零副作用。
//
// 运行：`npm run lint`（= `eslint src/net src/game-state src/protocol`）。
// 安装：`npm install`（package.json devDependencies 已列 eslint 与 @typescript-eslint/parser）。

import parser from "@typescript-eslint/parser";

/** 被守卫的目录（相对 eslint.config.js 所在的 web/）。 */
const GUARDED_DIRS = [
  "src/net/**/*.{js,ts}",
  "src/game-state/**/*.{js,ts}",
  "src/protocol/**/*.{js,ts}",
];

export default [
  // 不 lint 构建产物与依赖。
  { ignores: ["dist/**", "node_modules/**"] },

  // 守卫规则：仅在三个目录内禁止 import phaser（含子路径 import）。
  {
    files: GUARDED_DIRS,
    languageOptions: {
      parser,
      parserOptions: {
        sourceType: "module",
        ecmaVersion: 2022,
      },
    },
    rules: {
      "no-restricted-imports": [
        "error",
        {
          patterns: [
            {
              group: ["phaser", "phaser/*"],
              message:
                "net/、game-state/、protocol/ 不得 import phaser（前端分层禁令：CLAUDE.md / docs/development-plan.md §2.3）。phaser 只允许出现在 scene/ 与 src/main.ts。",
            },
          ],
        },
      ],
    },
  },
];
