/** The minimal SVG surface the exporter needs; a real SVGSVGElement matches. */
export interface SvgSurface {
  outerHTML: string;
  cloneNode(deep: boolean): SvgSurface;
  setAttribute(name: string, value: string): void;
}

/**
 * Serialize one rendered SVG with the namespace a standalone file needs.
 *
 * `XMLSerializer` is used in the browser; `outerHTML` is the fallback for a
 * host that does not expose it (the unit test environment).
 */
export function svgDocument(surface: SvgSurface): string {
  const clone = surface.cloneNode(true);
  clone.setAttribute("xmlns", "http://www.w3.org/2000/svg");
  if (typeof XMLSerializer !== "undefined") {
    return new XMLSerializer().serializeToString(clone as unknown as SVGSVGElement);
  }
  return clone.outerHTML;
}

/** The stable filename for one chart window. */
export function svgFilename(simulationId: string, chart: number): string {
  return `quantrule-${simulationId}-window-${chart}.svg`;
}

/** Download a serialized chart without adding a runtime dependency. */
export function downloadSvg(surface: SvgSurface, filename: string): void {
  const text = svgDocument(surface);
  const blob = new Blob([text], { type: "image/svg+xml" });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  anchor.click();
  URL.revokeObjectURL(url);
}
