import { afterAll, beforeEach, describe, expect, it } from "bun:test";
import { rememberToken, storedToken } from "./token";

const TOKEN = "qr_browser_token_0123456789abcdef";

function memoryStorage(): Storage {
  const values = new Map<string, string>();
  return {
    get length() {
      return values.size;
    },
    clear: () => values.clear(),
    getItem: (key: string) => values.get(key) ?? null,
    key: (index: number) => [...values.keys()][index] ?? null,
    removeItem: (key: string) => {
      values.delete(key);
    },
    setItem: (key: string, value: string) => {
      values.set(key, value);
    },
  };
}

const original = Object.getOwnPropertyDescriptor(globalThis, "localStorage");

function install(storage: Storage): void {
  Object.defineProperty(globalThis, "localStorage", {
    value: storage,
    configurable: true,
    writable: true,
  });
}

beforeEach(() => {
  install(memoryStorage());
});

afterAll(() => {
  if (original === undefined) {
    Reflect.deleteProperty(globalThis, "localStorage");
  } else {
    Object.defineProperty(globalThis, "localStorage", original);
  }
});

describe("the browser token store", () => {
  it("round-trips a token and trims it", () => {
    expect(storedToken()).toBeNull();
    expect(rememberToken(`  ${TOKEN}  `)).toBe(TOKEN);
    expect(storedToken()).toBe(TOKEN);
    expect(localStorage.getItem("quantrule.token")).toBe(TOKEN);
  });

  it("removes the token when asked to forget it", () => {
    rememberToken(TOKEN);
    expect(rememberToken(null)).toBeNull();
    expect(storedToken()).toBeNull();
    expect(localStorage.getItem("quantrule.token")).toBeNull();
  });

  it("treats a blank stored value as no token", () => {
    localStorage.setItem("quantrule.token", "   ");
    expect(storedToken()).toBeNull();
    expect(rememberToken("")).toBeNull();
  });

  it("keeps working when the browser blocks storage", () => {
    install({
      ...memoryStorage(),
      getItem: () => {
        throw new Error("storage is blocked");
      },
      setItem: () => {
        throw new Error("storage is blocked");
      },
    } as Storage);
    expect(storedToken()).toBeNull();
    expect(rememberToken(TOKEN)).toBe(TOKEN);
  });
});
