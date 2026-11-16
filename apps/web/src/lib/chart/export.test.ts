import { describe, expect, it } from "bun:test";
import { svgDocument, svgFilename, type SvgSurface } from "./export";

function surface(outerHTML: string): SvgSurface {
  const clone = {
    outerHTML,
    setAttribute: (name: string, value: string) => {
      clone.outerHTML = `<svg ${name}="${value}"><path d="M0 0"/></svg>`;
    },
    cloneNode: () => clone,
  };
  return {
    outerHTML,
    cloneNode: () => clone,
    setAttribute: () => undefined,
  };
}

describe("svgDocument", () => {
  it("returns a standalone SVG document with the namespace", () => {
    const rendered = surface('<svg viewBox="0 0 10 10"></svg>');
    expect(svgDocument(rendered)).toBe(
      '<svg xmlns="http://www.w3.org/2000/svg"><path d="M0 0"/></svg>',
    );
  });
});

describe("svgFilename", () => {
  it("names the simulation and window", () => {
    expect(svgFilename("sim-1", 3)).toBe("quantrule-sim-1-window-3.svg");
  });
});
