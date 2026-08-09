// ui/admin.test.ts — escapeHtml 纯函数单测（vitest）。无 phaser、无 DOM。
import { describe, expect, it } from "vitest";
import { escapeHtml } from "./admin";

describe("escapeHtml", () => {
  it("escapes < > & \" '", () => {
    expect(escapeHtml("<script>alert(\"x\")</script>")).toBe(
      "&lt;script&gt;alert(&quot;x&quot;)&lt;/script&gt;",
    );
    expect(escapeHtml("a & b")).toBe("a &amp; b");
    expect(escapeHtml("it's ok")).toBe("it&#39;s ok");
  });

  it("passes through plain text unchanged", () => {
    expect(escapeHtml("alice")).toBe("alice");
    expect(escapeHtml("user_name-01")).toBe("user_name-01");
  });

  it("escapes ampersand first to avoid double-escaping", () => {
    expect(escapeHtml("&lt;")).toBe("&amp;lt;");
    expect(escapeHtml("a&b<c>")).toBe("a&amp;b&lt;c&gt;");
  });

  it("handles empty string", () => {
    expect(escapeHtml("")).toBe("");
  });
});
