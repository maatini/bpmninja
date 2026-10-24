/**
 * Integration test against a real engine-server binary.
 *
 * The test is skipped when no binary is available. Build it with
 * `cargo build -p engine-server` or point `BPMNINJA_SERVER_BIN` at it.
 */

import { spawn, type ChildProcess } from "node:child_process";
import { existsSync } from "node:fs";
import net from "node:net";
import { fileURLToPath } from "node:url";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { ExternalTaskClient } from "../index.js";

const binaryName = process.platform === "win32" ? "engine-server.exe" : "engine-server";
const serverBin =
  process.env.BPMNINJA_SERVER_BIN ??
  fileURLToPath(new URL(`../../../target/debug/${binaryName}`, import.meta.url));

const hasServer = existsSync(serverBin);

if (!hasServer) {
  console.warn(
    `[integration] skipped: engine-server binary not found at ${serverBin}. ` +
      "Run `cargo build -p engine-server` or set BPMNINJA_SERVER_BIN.",
  );
}

function processXml(processId: string, topic: string): string {
  return `<?xml version="1.0" encoding="UTF-8"?>
<definitions id="Definitions_1" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
  <process id="${processId}" isExecutable="true">
    <startEvent id="start" />
    <serviceTask id="svc" data-topic="${topic}" />
    <endEvent id="end" />
    <sequenceFlow id="f1" sourceRef="start" targetRef="svc" />
    <sequenceFlow id="f2" sourceRef="svc" targetRef="end" />
  </process>
</definitions>`;
}

function freePort(): Promise<number> {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.on("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      const port = typeof address === "object" && address ? address.port : 0;
      server.close(() => resolve(port));
    });
  });
}

async function waitFor<T>(
  check: () => Promise<T | undefined | false>,
  timeoutMs = 15_000,
  intervalMs = 100,
): Promise<T> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const result = await check();
    if (result) {
      return result;
    }
    await new Promise((resolve) => setTimeout(resolve, intervalMs));
  }
  throw new Error(`Condition not met within ${timeoutMs}ms`);
}

describe.skipIf(!hasServer)("integration against engine-server", () => {
  let child: ChildProcess;
  let baseUrl: string;
  let serverLog = "";

  const jsonPost = async (path: string, body: unknown): Promise<Response> =>
    fetch(`${baseUrl}${path}`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });

  const deploy = async (xml: string, name: string): Promise<{ definition_key: string; version: number }> => {
    const response = await jsonPost("/api/deploy", { xml, name });
    expect(response.status).toBe(200);
    return (await response.json()) as { definition_key: string; version: number };
  };

  const start = async (definitionKey: string): Promise<string> => {
    const response = await jsonPost("/api/start", { definition_key: definitionKey });
    expect(response.status).toBe(200);
    const body = (await response.json()) as { instance_id: string };
    return body.instance_id;
  };

  const getInstance = async (id: string): Promise<Record<string, unknown>> => {
    const response = await fetch(`${baseUrl}/api/instances/${id}`);
    expect(response.status).toBe(200);
    return (await response.json()) as Record<string, unknown>;
  };

  const getIncidents = async (id: string): Promise<Record<string, unknown>[]> => {
    const response = await fetch(`${baseUrl}/api/instances/${id}/incidents`);
    expect(response.status).toBe(200);
    return (await response.json()) as Record<string, unknown>[];
  };

  beforeAll(async () => {
    const port = await freePort();
    baseUrl = `http://127.0.0.1:${port}`;

    child = spawn(serverBin, [], {
      env: {
        ...process.env,
        PORT: String(port),
        BIND_ADDR: "127.0.0.1",
        REQUIRE_NATS: "false",
        // Point at an unused port so the server always falls back to in-memory
        // state; otherwise a local NATS instance would make versions accumulate.
        NATS_URL: "nats://127.0.0.1:1",
        LOG_FILE: "off",
        RUST_LOG: "error",
      },
      stdio: ["ignore", "pipe", "pipe"],
    });
    child.stdout?.on("data", (chunk) => {
      serverLog += String(chunk);
    });
    child.stderr?.on("data", (chunk) => {
      serverLog += String(chunk);
    });

    try {
      await waitFor(async () => {
        const response = await fetch(`${baseUrl}/api/health`).catch(() => undefined);
        return response?.ok;
      });
    } catch (error) {
      throw new Error(
        `engine-server did not become healthy: ${(error as Error).message}\n${serverLog}`,
      );
    }
  }, 20_000);

  afterAll(async () => {
    if (!child || child.exitCode !== null) {
      return;
    }
    child.kill("SIGTERM");
    await new Promise<void>((resolve) => {
      const timeout = setTimeout(() => {
        child.kill("SIGKILL");
        resolve();
      }, 5_000);
      child.once("exit", () => {
        clearTimeout(timeout);
        resolve();
      });
    });
  });

  it(
    "completes a service task through the client",
    async () => {
      const deployed = await deploy(processXml("IntegrationComplete", "integration_complete"), "integration complete");
      // DeployResponse carries the version (part of the OpenAPI contract).
      expect(deployed.version).toBe(1);

      const instanceId = await start(deployed.definition_key);
      const client = new ExternalTaskClient({
        baseUrl,
        logger: false,
        pollingInterval: 50,
        asyncResponseTimeout: 0,
        requestTimeout: 5_000,
        requestMaxRetries: 2,
        maxRetries: 0,
      });

      let completed = false;
      client.subscribe("integration_complete", async (_task, service) => {
        await service.complete({ result: 42 });
        completed = true;
      });
      client.start();

      try {
        const instance = await waitFor(async () => {
          const current = await getInstance(instanceId);
          return current.state === "Completed" ? current : undefined;
        });
        expect(completed).toBe(true);
        expect((instance.variables as Record<string, unknown>).result).toBe(42);
      } finally {
        await client.stop();
      }
    },
    30_000,
  );

  it(
    "reports a handler failure as an incident",
    async () => {
      const deployed = await deploy(processXml("IntegrationFail", "integration_fail"), "integration fail");
      const instanceId = await start(deployed.definition_key);
      const client = new ExternalTaskClient({
        baseUrl,
        logger: false,
        pollingInterval: 50,
        asyncResponseTimeout: 0,
        requestTimeout: 5_000,
        requestMaxRetries: 2,
        maxRetries: 0,
      });

      client.subscribe("integration_fail", async () => {
        throw new Error("worker boom");
      });
      client.start();

      try {
        const incidents = await waitFor(async () => {
          const current = await getIncidents(instanceId);
          return current.length > 0 ? current : undefined;
        });
        expect(incidents[0].kind).toBe("service_task");
        expect(String(incidents[0].message)).toContain("worker boom");
      } finally {
        await client.stop();
      }
    },
    30_000,
  );

  it("starts the latest definition version and returns its key and version", async () => {
    const first = await deploy(processXml("IntegrationLatest", "integration_latest"), "latest v1");
    const second = await deploy(processXml("IntegrationLatest", "integration_latest"), "latest v2");
    expect(first.version).toBe(1);
    expect(second.version).toBe(2);

    const response = await jsonPost("/api/start/latest", {
      bpmn_process_id: "IntegrationLatest",
    });
    expect(response.status).toBe(200);

    const body = (await response.json()) as {
      instance_id: string;
      definition_key: string;
      version: number;
    };
    expect(body.instance_id).toBeTypeOf("string");
    expect(body.definition_key).toBe(second.definition_key);
    expect(body.version).toBe(2);
  }, 20_000);
});
