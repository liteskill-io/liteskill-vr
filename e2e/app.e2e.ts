import { browser, expect } from "@wdio/globals";
import fs from "node:fs";
import path from "node:path";

const MCP_URL = "http://127.0.0.1:27182/mcp";
const SCREENSHOT_DIR = "e2e/screenshots";
const HERO_SCREENSHOT = path.join("src", "assets", "dashboard.png");

// --- MCP helpers -----------------------------------------------------------

interface RpcContent {
  type: string;
  text: string;
}
interface RpcResult {
  content: RpcContent[];
}
interface RpcResponse {
  jsonrpc: "2.0";
  id: number;
  result?: RpcResult;
  error?: { code: number; message: string };
}

let rpcId = 0;

async function rpc(method: string, params: unknown): Promise<RpcResponse> {
  rpcId += 1;
  const resp = await fetch(MCP_URL, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Accept: "application/json, text/event-stream",
      "X-LiteSkill-Author": "claude-code",
    },
    body: JSON.stringify({ jsonrpc: "2.0", id: rpcId, method, params }),
  });
  if (!resp.ok) {
    throw new Error(`MCP ${method} failed: HTTP ${String(resp.status)}`);
  }
  return (await resp.json()) as RpcResponse;
}

async function tool<T = unknown>(
  name: string,
  args: Record<string, unknown>,
): Promise<T> {
  const resp = await rpc("tools/call", { name, arguments: args });
  if (resp.error) {
    throw new Error(`MCP tool ${name}: ${resp.error.message}`);
  }
  const text = resp.result?.content[0]?.text ?? "null";
  return JSON.parse(text) as T;
}

async function waitForMcp(timeoutMs = 30_000): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      const resp = await rpc("initialize", {
        protocolVersion: "2024-11-05",
        capabilities: {},
        clientInfo: { name: "e2e", version: "1.0" },
      });
      if (resp.result) return;
    } catch {
      // still booting — retry
    }
    await new Promise((r) => setTimeout(r, 200));
  }
  throw new Error("MCP server did not become ready in time");
}

// --- Seed data -------------------------------------------------------------

interface ItemResult {
  id: string;
  name: string;
}

// Item id by name, populated by seedDemoProject so later seeders (explanations)
// can scope their content to real items.
const seededItems: Record<string, string> = {};

async function seedDemoProject(): Promise<void> {
  const items = await tool<ItemResult[]>("item_create_batch", {
    items: [
      {
        name: "httpd",
        item_type: "elf",
        path: "/usr/sbin/httpd",
        architecture: "x86_64",
        description:
          "Custom fork of Apache 2.4 — primary attack surface. Vendor patches in request parsing and session handling.",
      },
      {
        name: "libssl.so",
        item_type: "shared_object",
        path: "/usr/lib/libssl.so.1.1",
        description: "OpenSSL 1.1.1 fork with modified cert verification.",
      },
      {
        name: "libcrypto.so",
        item_type: "shared_object",
        path: "/usr/lib/libcrypto.so.1.1",
      },
      {
        name: "auth_daemon",
        item_type: "elf",
        path: "/usr/sbin/auth_daemon",
        description: "LDAP-backed session authority.",
      },
      {
        name: "update_client",
        item_type: "elf",
        path: "/usr/bin/update_client",
        description: "Periodic firmware updater. Runs as root.",
      },
      { name: "nginx.conf", item_type: "config", path: "/etc/nginx.conf" },
      { name: "libjson.so", item_type: "shared_object" },
    ],
  });
  for (const it of items) seededItems[it.name] = it.id;
  const id = (n: string): string => {
    const v = seededItems[n];
    if (!v) throw new Error(`seed: missing item '${n}'`);
    return v;
  };

  await tool("ioi_create_batch", {
    item_id: id("httpd"),
    items: [
      {
        title: "parse_request()",
        description:
          "Stack buffer overflow parsing Content-Length. Overflows a 256-byte buffer with attacker-controlled data.",
        location: "0x08041234",
        severity: "critical",
        tags: ["memory-corruption"],
      },
      {
        title: "handle_cookie()",
        description:
          "Use-after-free on cookie struct after session timeout. Reachable without auth.",
        location: "0x080429a0",
        severity: "critical",
        tags: ["memory-corruption"],
      },
      {
        title: "log_rotate()",
        description:
          "TOCTOU between stat() and open() enables symlink attack during rotation.",
        location: "0x08043510",
        severity: "high",
        tags: ["race-condition"],
      },
      {
        title: "md5_cert_chain()",
        description:
          "Accepts MD5-signed intermediates during chain verification.",
        location: "0x08043c20",
        severity: "medium",
        tags: ["crypto-weakness"],
      },
    ],
  });

  await tool("ioi_create_batch", {
    item_id: id("libssl.so"),
    items: [
      {
        title: "check_signature()",
        description:
          "RSA signature verification leaks timing via early-exit on length check.",
        location: "libssl!0x14a0",
        severity: "high",
        tags: ["crypto-weakness"],
      },
      {
        title: "parse_x509_ext()",
        description: "Heap overflow on malformed certificate extension.",
        location: "libssl!0x28f0",
        severity: "medium",
        tags: ["memory-corruption"],
      },
    ],
  });

  await tool("ioi_create_batch", {
    item_id: id("auth_daemon"),
    items: [
      {
        title: "auth_ldap()",
        description:
          "Empty password bypasses LDAP bind — Anonymous Bind accepted by default.",
        location: "0x040218b0",
        severity: "critical",
        tags: ["auth-bypass"],
      },
      {
        title: "verify_token()",
        description:
          "strcmp() enables timing side-channel leak of valid token.",
        location: "0x04023100",
        severity: "high",
        tags: ["auth-bypass"],
      },
      {
        title: "debug_cmd_handler()",
        description:
          "Hidden /admin/debug route accepts raw shell commands, no auth check.",
        location: "0x04024a40",
        severity: "critical",
        tags: ["debug-interface"],
      },
    ],
  });

  await tool("ioi_create_batch", {
    item_id: id("update_client"),
    items: [
      {
        title: "update_fetcher()",
        description:
          "Downloads firmware over plain HTTP; signature verification optional via env var.",
        location: "0x00401980",
        severity: "critical",
        tags: ["crypto-weakness"],
      },
      {
        title: "parse_manifest()",
        description:
          "Integer overflow on chunk-length triggers heap corruption.",
        location: "0x00402a10",
        severity: "high",
        tags: ["integer-issue"],
      },
    ],
  });

  await tool("ioi_create", {
    item_id: id("libcrypto.so"),
    title: "rand_seed()",
    description:
      "PRNG seeded from PID + time; predictable in containerized envs.",
    location: "libcrypto!0x3f10",
    severity: "low",
    tags: ["crypto-weakness"],
  });
  await tool("ioi_create", {
    item_id: id("libjson.so"),
    title: "json_escape()",
    description:
      "CRLF injection via escaped newlines when embedded in HTTP headers.",
    severity: "info",
    tags: ["info-disclosure"],
  });

  await tool("note_create_batch", {
    notes: [
      {
        item_id: id("httpd"),
        title: "Scope & Methodology",
        content:
          "httpd is a fork of Apache 2.4.54 with vendor patches. Focus on request parsing and session handling. Running Ghidra decompilation in parallel via pyghidra-mcp.",
      },
      {
        item_id: id("libssl.so"),
        title: "TLS stack overview",
        content:
          "libssl wraps libcrypto with custom cert validation. The chain walker diverges from upstream — audit separately.",
      },
    ],
  });

  await tool("connection_create_batch", {
    connections: [
      {
        source_id: id("httpd"),
        source_type: "item",
        target_id: id("libssl.so"),
        target_type: "item",
        connection_type: "links",
        description: "Dynamically links libssl for TLS termination",
      },
      {
        source_id: id("httpd"),
        source_type: "item",
        target_id: id("libcrypto.so"),
        target_type: "item",
        connection_type: "links",
        description: "",
      },
      {
        source_id: id("httpd"),
        source_type: "item",
        target_id: id("libjson.so"),
        target_type: "item",
        connection_type: "links",
        description: "",
      },
      {
        source_id: id("httpd"),
        source_type: "item",
        target_id: id("nginx.conf"),
        target_type: "item",
        connection_type: "reads_config",
        description: "",
      },
      {
        source_id: id("auth_daemon"),
        source_type: "item",
        target_id: id("libssl.so"),
        target_type: "item",
        connection_type: "links",
        description: "",
      },
      {
        source_id: id("update_client"),
        source_type: "item",
        target_id: id("libssl.so"),
        target_type: "item",
        connection_type: "links",
        description: "",
      },
      {
        source_id: id("update_client"),
        source_type: "item",
        target_id: id("libjson.so"),
        target_type: "item",
        connection_type: "links",
        description: "",
      },
    ],
  });

  // Mix of statuses: 2 reviewed, 4 in_progress, 1 untouched.
  await tool("item_update", {
    id: id("libcrypto.so"),
    analysis_status: "reviewed",
  });
  await tool("item_update", {
    id: id("libjson.so"),
    analysis_status: "reviewed",
  });
  for (const name of ["httpd", "libssl.so", "auth_daemon", "update_client"]) {
    await tool("item_update", { id: id(name), analysis_status: "in_progress" });
  }
}

// Seed the typed-content explanations that the new features render: a state
// machine (states + transitions → on-the-fly diagram), a packet format
// (structured fields → byte-layout table), and a protocol with a sanitized
// HTML diagram. States/transitions/fields/diagram_html are all accepted inline
// by explanation_upsert.
async function seedExplanations(): Promise<void> {
  const httpd = seededItems["httpd"];
  const scope = httpd ? [httpd] : [];

  // State machine — drives the custom (non-Mermaid) on-the-fly diagram.
  await tool("explanation_upsert", {
    stable_key: "explanation.auth_flow",
    title: "Authentication flow",
    explanation_type: "state_machine",
    summary:
      "Client sends HELLO; server returns a nonce; client returns a signed nonce; on valid signature the session moves to AUTHENTICATED and privileged commands are accepted.",
    confidence: "medium",
    scope_item_ids: scope,
    states: [
      {
        stable_key: "UNAUTH",
        name: "UNAUTHENTICATED",
        description: "No session key; only HELLO is accepted.",
        is_initial: true,
      },
      {
        stable_key: "CHALLENGE",
        name: "CHALLENGE_SENT",
        description:
          "Server has issued a nonce and is awaiting a signed response.",
      },
      {
        stable_key: "AUTHED",
        name: "AUTHENTICATED",
        description: "Privileged commands are accepted.",
        is_terminal: true,
      },
    ],
    transitions: [
      {
        stable_key: "t.hello",
        from_state: "UNAUTH",
        to_state: "CHALLENGE",
        event: "HELLO",
        action: "issue nonce",
      },
      {
        stable_key: "t.signed",
        from_state: "CHALLENGE",
        to_state: "AUTHED",
        event: "SIGNED_NONCE",
        guard: "signature valid",
        action: "derive session key",
      },
      {
        stable_key: "t.bad",
        from_state: "CHALLENGE",
        to_state: "UNAUTH",
        event: "SIGNED_NONCE",
        guard: "signature invalid",
      },
    ],
    claims: [
      {
        stable_key: "claim.rsa",
        text: "Signature verification uses RSA-2048 against a baked-in public key.",
        claim_type: "behavior",
        status: "supported",
        confidence: "high",
      },
    ],
    open_questions: [
      {
        stable_key: "q.bound",
        question: "Is LoginRequest.length bounded before the payload copy?",
        priority: "high",
      },
    ],
  });

  // Packet format — structured fields → dedicated byte-layout renderer.
  await tool("explanation_upsert", {
    stable_key: "explanation.packet_login",
    title: "LoginRequest packet format",
    explanation_type: "packet_format",
    summary:
      "magic(u32)=0x4c534b4c, type(u8)=3, length(u16, payload bytes), payload(bytes). length and payload are attacker-controlled.",
    confidence: "medium",
    scope_item_ids: scope,
    fields: [
      {
        stable_key: "magic",
        name: "magic",
        field_type: "u32",
        offset: 0,
        size: 4,
        description: "Must equal 0x4c534b4c",
      },
      {
        stable_key: "type",
        name: "type",
        field_type: "u8",
        offset: 4,
        size: 1,
        description: "3 = LOGIN",
      },
      {
        stable_key: "length",
        name: "length",
        field_type: "u16",
        offset: 5,
        size: 2,
        description: "Attacker-controlled payload length",
      },
      {
        stable_key: "payload",
        name: "payload",
        field_type: "bytes",
        offset: 7,
        description: "Copied into a 256-byte stack buffer",
      },
    ],
    claims: [
      {
        stable_key: "claim.len_attacker",
        text: "length is used directly as the memcpy size in parse_login().",
        claim_type: "security_relevant",
        status: "supported",
        confidence: "high",
      },
    ],
  });

  // Protocol — agent-authored HTML diagram, sanitized server-side on write.
  await tool("explanation_upsert", {
    stable_key: "explanation.protocol",
    title: "Wire protocol overview",
    explanation_type: "protocol",
    summary:
      "Length-prefixed binary protocol: 4-byte magic, 1-byte type, 2-byte length, payload. Types: HELLO(1), LOGIN(3), CMD(4).",
    confidence: "medium",
    scope_item_ids: scope,
    diagram_html:
      "<table><thead><tr><th>Type</th><th>Name</th><th>Direction</th><th>Payload</th></tr></thead><tbody>" +
      "<tr><td>1</td><td>HELLO</td><td>client → server</td><td>none</td></tr>" +
      "<tr><td>2</td><td>NONCE</td><td>server → client</td><td>16-byte nonce</td></tr>" +
      "<tr><td>3</td><td>LOGIN</td><td>client → server</td><td>signed nonce + creds</td></tr>" +
      "<tr><td>4</td><td>CMD</td><td>client → server</td><td>command (post-auth)</td></tr>" +
      "</tbody></table>",
    claims: [
      {
        stable_key: "claim.framing",
        text: "Messages are framed by the 2-byte length field; there is no overall packet checksum.",
        claim_type: "behavior",
        status: "supported",
        confidence: "medium",
      },
    ],
  });
}

// --- Test setup ------------------------------------------------------------

function screenshotName(title: string): string {
  return (
    title
      .replace(/\s+/g, "-")
      .replace(/[^a-z0-9-]/gi, "")
      .toLowerCase() + ".png"
  );
}

describe("LiteSkill VR", () => {
  before(async () => {
    // Make screenshots large enough to look good on a README.
    await browser.setWindowSize(1400, 900);
    await waitForMcp();
  });

  afterEach(async function () {
    const title = this.currentTest?.title;
    if (!title) return;
    await browser.saveScreenshot(
      path.join(SCREENSHOT_DIR, screenshotName(title)),
    );
  });

  it("boots with an empty dashboard", async () => {
    // Unqualified `*=foo` is "partial link text" in WebDriver — only matches
    // <a> tags. Tag-qualify every text selector to match any element.
    await expect(browser.$("div*=LITESKILL")).toBeDisplayed();
    await expect(browser.$("div*=MCP server running")).toBeDisplayed();
  });

  describe("after seeding a demo project", () => {
    before(async () => {
      await seedDemoProject();
      // Wait for db-changed → snapshot refetch → DOM update.
      await browser.waitUntil(
        async () => {
          const el = await browser.$("div*=Critical & High");
          return el.isExisting();
        },
        {
          timeout: 20_000,
          interval: 250,
          timeoutMsg: "dashboard never populated with seeded data",
        },
      );
      await browser.pause(400);
    });

    it("renders the populated dashboard", async () => {
      await expect(browser.$("div*=Severity Breakdown")).toBeDisplayed();
      await expect(browser.$("div*=Triage Status")).toBeDisplayed();
      await expect(browser.$("div*=Analysis Progress")).toBeDisplayed();
      await expect(browser.$("div*=Critical & High")).toBeDisplayed();
      await expect(browser.$("div*=Recent Findings")).toBeDisplayed();

      // Committed hero screenshot for the README.
      fs.mkdirSync(path.dirname(HERO_SCREENSHOT), { recursive: true });
      await browser.saveScreenshot(HERO_SCREENSHOT);
    });

    it("shows all seeded items in the sidebar", async () => {
      await expect(browser.$("div*=All Items")).toBeDisplayed();
      await expect(browser.$("button*=httpd")).toBeDisplayed();
      await expect(browser.$("button*=auth_daemon")).toBeDisplayed();
      await expect(browser.$("button*=update_client")).toBeDisplayed();
    });

    it("groups findings by severity in the sidebar", async () => {
      await expect(browser.$("div*=By Severity")).toBeDisplayed();
      await expect(browser.$("span*=critical")).toBeDisplayed();
      await expect(browser.$("span*=high")).toBeDisplayed();
    });

    it("opens an item detail when its sidebar entry is clicked", async () => {
      const httpdBtn = await browser.$("button*=httpd");
      // WebKitGTK's WebDriver doesn't implement the native click endpoint,
      // so dispatch the click via script.
      await browser.execute((el: HTMLElement) => {
        el.click();
      }, httpdBtn);
      await browser.pause(300);

      await expect(browser.$("h1")).toHaveText("httpd");
      await expect(browser.$("div*=Items of Interest")).toBeDisplayed();
      await expect(browser.$("div*=Notes")).toBeDisplayed();
      await expect(browser.$("div*=Connections")).toBeDisplayed();
      await expect(browser.$("span*=parse_request()")).toBeDisplayed();
    });

    it("renders the connection map when the sidebar button is clicked", async () => {
      const mapBtn = await browser.$("button*=Connection Map");
      await browser.execute((el: HTMLElement) => {
        el.click();
      }, mapBtn);
      await browser.waitUntil(
        async () => {
          const el = await browser.$("span*=Connection Map");
          return el.isExisting();
        },
        { timeout: 5000, timeoutMsg: "connection map never rendered" },
      );
      // Toolbar controls land alongside the header.
      await expect(browser.$("button*=Fit")).toBeDisplayed();
      await expect(browser.$("button*=Layout")).toBeDisplayed();
      // cose layout is synchronous (animate:false) but give cytoscape a tick
      // to finish painting.
      await browser.pause(500);
      await expect(browser.$("span*=7 items")).toBeDisplayed();
      await expect(browser.$("span*=7 connections")).toBeDisplayed();
    });

    describe("explanations knowledge layer", () => {
      // WebKitGTK's WebDriver doesn't implement the native click endpoint, so
      // every click is dispatched via script.
      async function click(selector: string): Promise<void> {
        const el = await browser.$(selector);
        await el.waitForExist({ timeout: 5000 });
        await browser.execute((node: HTMLElement) => {
          node.click();
        }, el);
        await browser.pause(300);
      }

      async function openSection(): Promise<void> {
        await click("button*=Explanations");
        await browser.waitUntil(
          async () => {
            const el = await browser.$("button*=Authentication flow");
            return el.isExisting();
          },
          { timeout: 10_000, timeoutMsg: "explanations list never populated" },
        );
      }

      before(async () => {
        await seedExplanations();
        // Wait for db-changed → snapshot refetch before navigating.
        await browser.pause(600);
        await openSection();
      });

      it("lists explanations in the knowledge layer", async () => {
        await expect(browser.$("h1*=Explanations")).toBeDisplayed();
        await expect(browser.$("button*=Authentication flow")).toBeDisplayed();
        await expect(
          browser.$("button*=LoginRequest packet format"),
        ).toBeDisplayed();
        await expect(
          browser.$("button*=Wire protocol overview"),
        ).toBeDisplayed();
      });

      it("renders a state machine with an on-the-fly diagram", async () => {
        await click("button*=Authentication flow");
        await expect(browser.$("h1*=Authentication flow")).toBeDisplayed();
        // SectionHeader: "State machine (3 states, 3 transitions)".
        await expect(browser.$("h2*=State machine")).toBeDisplayed();
        // States and a transition label rendered from the editable rows.
        // (Tag-qualify text selectors — bare `*=` only matches <a> tags.)
        await expect(browser.$("span*=UNAUTHENTICATED")).toBeDisplayed();
        await expect(browser.$("span*=AUTHENTICATED")).toBeDisplayed();
        await expect(browser.$("span*=SIGNED_NONCE")).toBeDisplayed();
        await browser.pause(300);
      });

      it("renders a packet format as a structured field table", async () => {
        await click("button*=All explanations");
        await click("button*=LoginRequest packet format");
        await expect(
          browser.$("h1*=LoginRequest packet format"),
        ).toBeDisplayed();
        // The dedicated byte-layout renderer: header + field rows.
        await expect(browser.$("h2*=Structure")).toBeDisplayed();
        await expect(browser.$("th*=Bytes")).toBeDisplayed();
        await expect(browser.$("td*=magic")).toBeDisplayed();
        await expect(browser.$("td*=payload")).toBeDisplayed();
        await browser.pause(300);
      });

      it("renders a sanitized HTML diagram", async () => {
        await click("button*=All explanations");
        await click("button*=Wire protocol overview");
        await expect(browser.$("h1*=Wire protocol overview")).toBeDisplayed();
        await expect(browser.$("h2*=Diagram")).toBeDisplayed();
        // The sanitized table is rendered (scripts stripped server-side). On the
        // protocol detail the only <td> cells come from the diagram table.
        await expect(browser.$("td*=HELLO")).toBeDisplayed();
        await browser.pause(300);
      });
    });
  });
});
