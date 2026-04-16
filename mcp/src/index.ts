#!/usr/bin/env node

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { execFile } from "node:child_process";
import { existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { promisify } from "node:util";
import { z } from "zod";

const execFileAsync = promisify(execFile);

/** Find the decay binary: same-repo build → system PATH */
function findDecayBinary(): string {
  const serverDir = dirname(new URL(import.meta.url).pathname);
  const projectRoot = resolve(serverDir, "..", "..");

  const candidates = [
    join(projectRoot, "target", "release", "decay"),
    join(projectRoot, "target", "debug", "decay"),
  ];

  for (const candidate of candidates) {
    if (existsSync(candidate)) {
      return candidate;
    }
  }

  return "decay"; // fallback to PATH
}

const DECAY_BIN = findDecayBinary();

const server = new McpServer({
  name: "decay",
  version: "0.1.0",
});

server.tool(
  "decay_check",
  "Run project health check. Returns summary (headline + narrative + top 3 actions) first, then full diagnostic report with 8 dimension scores, classified issues, impact assessment, patches, and preventions.",
  {
    path: z.string().optional().describe("Project path (default: current working directory)"),
  },
  async ({ path }) => {
    const cwd = path ? resolve(path) : process.cwd();

    try {
      const { stdout } = await execFileAsync(DECAY_BIN, ["--json"], {
        cwd,
        timeout: 60000,
      });

      const result = JSON.parse(stdout);

      // Return summary as first content block for quick consumption,
      // full report as second block for details
      const summary = result.summary;
      const summaryText = summary
        ? `${summary.headline}\n\n${summary.narrative}\n\nTop actions:\n${(summary.top_actions || []).map((a: { priority: string; what: string; effort: string }) => `  [${a.priority}] ${a.what} (${a.effort})`).join("\n")}`
        : "No summary available.";

      return {
        content: [
          {
            type: "text" as const,
            text: summaryText,
          },
        ],
      };
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : String(error);
      return {
        content: [
          {
            type: "text" as const,
            text: `Error running decay: ${message}`,
          },
        ],
        isError: true,
      };
    }
  }
);

async function main() {
  const transport = new StdioServerTransport();
  await server.connect(transport);
}

main().catch((error) => {
  console.error("Fatal error:", error);
  process.exit(1);
});
