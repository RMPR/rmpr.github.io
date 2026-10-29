// Azure Functions entry point: every /api/* request goes to handlers.js.

import { app } from "@azure/functions";
import { handle } from "../handlers.js";

app.http("api", {
  methods: ["GET", "POST"],
  authLevel: "anonymous", // Static Web Apps does the sign-in; see handlers.js
  route: "{*path}",
  handler: async (request) => {
    const url = new URL(request.url);
    const body = request.method === "POST" ? await request.json().catch(() => ({})) : undefined;
    const res = await handle({
      method: request.method,
      path: request.params.path || "",
      headers: Object.fromEntries(request.headers),
      query: url.searchParams,
      body,
    });
    return { status: res.status, headers: res.headers, body: res.body };
  },
});
