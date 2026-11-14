/**
 * A gateway stub for the Linux smoke test.
 *
 * It answers the two requests the shell makes before the web client exists:
 * `/health` for the reachability probe and `/` for the navigation target. The
 * page is deliberately not the built client: the smoke test proves the shell
 * reached the gateway URL, not that the client renders.
 *
 * Run with `bun tests/gateway-stub.js`; the process prints one line per request.
 */

const hostname = "::";
const port = Number(process.env.QR_SMOKE_PORT ?? "8080");

const server = Bun.serve({
  hostname,
  port,
  fetch(request) {
    const url = new URL(request.url);
    console.log(`${request.method} ${url.pathname}`);
    if (url.pathname === "/health") {
      return Response.json({ status: "ok" });
    }
    if (url.pathname === "/" || url.pathname === "/index.html") {
      return new Response(
        "<!doctype html><meta charset=\"utf-8\"><title>gateway smoke</title><p id=\"gateway-smoke\">gateway reached</p>",
        { headers: { "content-type": "text/html; charset=utf-8" } },
      );
    }
    return new Response("not found", { status: 404 });
  },
});

console.log(`gateway-stub listening on http://127.0.0.1:${server.port}`);

for (const signal of ["SIGINT", "SIGTERM"]) {
  process.on(signal, () => {
    server.stop(true);
    process.exit(0);
  });
}
