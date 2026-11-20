import { describe, expect, test } from "bun:test";

import { SecureContextRequiredError, sha256Hex, sha256HexWithSubtle } from "./artifacts";

describe("artifact hashing", () => {
  test("a non-secure context gets a clear error", async () => {
    await expect(sha256HexWithSubtle(new Uint8Array([1, 2, 3]), undefined)).rejects.toBeInstanceOf(
      SecureContextRequiredError,
    );
    await expect(sha256HexWithSubtle(new Uint8Array([1, 2, 3]), undefined)).rejects.toThrow(
      "HTTPS or localhost",
    );
  });

  test("a secure context hashes the bytes", async () => {
    expect(await sha256Hex(new TextEncoder().encode("abc"))).toBe(
      "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
    );
  });
});
