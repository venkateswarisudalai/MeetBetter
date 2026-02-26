/**
 * Vantage API Proxy — Cloudflare Worker
 *
 * Holds Deepgram + Groq API keys server-side so users can try the app
 * without obtaining their own keys. Includes per-IP daily rate limiting
 * and usage analytics logged to Supabase.
 *
 * Endpoints:
 *   GET  /api/deepgram/token  — returns a temporary Deepgram API key (5 min TTL)
 *   POST /api/groq/chat       — proxies to Groq chat completions
 *   POST /api/groq/whisper    — proxies to Groq Whisper transcription
 *   POST /api/heartbeat       — device registration and event logging
 *   GET  /health              — health check
 *
 * Secrets (set via `wrangler secret put <NAME>`):
 *   GROQ_API_KEY, DEEPGRAM_API_KEY, DEEPGRAM_PROJECT_ID,
 *   SUPABASE_URL, SUPABASE_ANON_KEY
 */

const CORS_HEADERS = {
  "Access-Control-Allow-Origin": "*",
  "Access-Control-Allow-Methods": "GET, POST, OPTIONS",
  "Access-Control-Allow-Headers": "Content-Type, X-Device-ID",
};

function corsResponse(body, status = 200, extraHeaders = {}) {
  return new Response(body, {
    status,
    headers: { ...CORS_HEADERS, "Content-Type": "application/json", ...extraHeaders },
  });
}

function errorResponse(message, status = 400) {
  return corsResponse(JSON.stringify({ error: message }), status);
}

// ---- Rate Limiting ----

async function checkRateLimit(env, ip, bucket, limit) {
  const today = new Date().toISOString().split("T")[0];
  const key = `rate:${bucket}:${ip}:${today}`;

  const current = parseInt((await env.RATE_LIMIT.get(key)) || "0", 10);
  if (current >= limit) {
    return { allowed: false, remaining: 0 };
  }

  await env.RATE_LIMIT.put(key, String(current + 1), { expirationTtl: 86400 });
  return { allowed: true, remaining: limit - current - 1 };
}

// ---- Privacy-safe IP hashing ----

async function hashIP(ip) {
  const today = new Date().toISOString().split("T")[0];
  const data = new TextEncoder().encode(`${ip}:${today}:vantage-salt-2024`);
  const hashBuffer = await crypto.subtle.digest("SHA-256", data);
  const hashArray = Array.from(new Uint8Array(hashBuffer));
  return hashArray.map(b => b.toString(16).padStart(2, "0")).join("").substring(0, 16);
}

// ---- Analytics ----

async function logAnalytics(env, { ipHash, endpoint, method, status, durationMs, deviceId }) {
  if (!env.SUPABASE_URL || !env.SUPABASE_ANON_KEY) return;

  try {
    await fetch(`${env.SUPABASE_URL}/rest/v1/proxy_analytics`, {
      method: "POST",
      headers: {
        "apikey": env.SUPABASE_ANON_KEY,
        "Authorization": `Bearer ${env.SUPABASE_ANON_KEY}`,
        "Content-Type": "application/json",
        "Prefer": "return=minimal",
      },
      body: JSON.stringify({
        ip_hash: ipHash,
        endpoint,
        method,
        response_status: status,
        duration_ms: durationMs,
        device_id: deviceId || null,
        created_at: new Date().toISOString(),
      }),
    });
  } catch (err) {
    console.error("Analytics log failed:", err.message);
  }
}

// ---- Heartbeat / Device Registration ----

async function handleHeartbeat(request, env, ip) {
  let body;
  try {
    body = await request.json();
  } catch {
    return errorResponse("Invalid JSON body");
  }

  const deviceId = body.device_id;
  if (!deviceId || typeof deviceId !== "string" || deviceId.length > 36) {
    return errorResponse("Invalid device_id");
  }

  if (!env.SUPABASE_URL || !env.SUPABASE_ANON_KEY) {
    return corsResponse(JSON.stringify({ status: "ok", message: "analytics disabled" }));
  }

  const ipHash = await hashIP(ip);
  const now = new Date().toISOString();

  // Upsert device record
  try {
    await fetch(`${env.SUPABASE_URL}/rest/v1/devices`, {
      method: "POST",
      headers: {
        "apikey": env.SUPABASE_ANON_KEY,
        "Authorization": `Bearer ${env.SUPABASE_ANON_KEY}`,
        "Content-Type": "application/json",
        "Prefer": "resolution=merge-duplicates,return=minimal",
      },
      body: JSON.stringify({
        device_id: deviceId,
        ip_hash: ipHash,
        os: body.os || null,
        arch: body.arch || null,
        app_version: body.app_version || null,
        last_seen: now,
        first_seen: now,
      }),
    });
  } catch (err) {
    console.error("Device upsert failed:", err.message);
  }

  // Log event
  try {
    await fetch(`${env.SUPABASE_URL}/rest/v1/events`, {
      method: "POST",
      headers: {
        "apikey": env.SUPABASE_ANON_KEY,
        "Authorization": `Bearer ${env.SUPABASE_ANON_KEY}`,
        "Content-Type": "application/json",
        "Prefer": "return=minimal",
      },
      body: JSON.stringify({
        device_id: deviceId,
        event_type: body.event || "app_open",
        ip_hash: ipHash,
        metadata: {
          app_version: body.app_version,
          os: body.os,
          arch: body.arch,
        },
        created_at: now,
      }),
    });
  } catch (err) {
    console.error("Event log failed:", err.message);
  }

  return corsResponse(JSON.stringify({ status: "ok" }));
}

// ---- Deepgram Temporary Key ----

async function handleDeepgramToken(env, ip) {
  const limit = parseInt(env.DAILY_DEEPGRAM_TOKEN_LIMIT || "10", 10);
  const { allowed, remaining } = await checkRateLimit(env, ip, "dg-token", limit);

  if (!allowed) {
    return errorResponse("Daily demo limit reached. Please try again tomorrow or add your own Deepgram key in Settings.", 429);
  }

  // Create a short-lived scoped key via Deepgram API
  const resp = await fetch(
    `https://api.deepgram.com/v1/projects/${env.DEEPGRAM_PROJECT_ID}/keys`,
    {
      method: "POST",
      headers: {
        Authorization: `Token ${env.DEEPGRAM_API_KEY}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        comment: `demo-${ip.replace(/[.:]/g, "-")}-${Date.now()}`,
        scopes: ["usage:write"],
        time_to_live_in_seconds: 300, // 5 minutes
      }),
    }
  );

  if (!resp.ok) {
    const errText = await resp.text();
    console.error("Deepgram key creation failed:", resp.status, errText);
    return errorResponse("Failed to create temporary key", 502);
  }

  const data = await resp.json();
  return corsResponse(
    JSON.stringify({
      key: data.key,
      expires_in: 300,
      remaining_today: remaining,
    })
  );
}

// ---- Groq Chat Proxy ----

async function handleGroqChat(request, env, ip) {
  const limit = parseInt(env.DAILY_LIMIT || "50", 10);
  const { allowed, remaining } = await checkRateLimit(env, ip, "groq-chat", limit);

  if (!allowed) {
    return errorResponse("Daily demo limit reached. Please try again tomorrow or add your own Groq key in Settings.", 429);
  }

  let body;
  try {
    body = await request.json();
  } catch {
    return errorResponse("Invalid JSON body");
  }

  // Safety: cap max_tokens to prevent abuse
  if (body.max_tokens && body.max_tokens > 2048) {
    body.max_tokens = 2048;
  }

  const resp = await fetch("https://api.groq.com/openai/v1/chat/completions", {
    method: "POST",
    headers: {
      Authorization: `Bearer ${env.GROQ_API_KEY}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify(body),
  });

  const data = await resp.text();
  return new Response(data, {
    status: resp.status,
    headers: {
      ...CORS_HEADERS,
      "Content-Type": "application/json",
      "X-RateLimit-Remaining": String(remaining),
    },
  });
}

// ---- Groq Whisper Proxy ----

async function handleGroqWhisper(request, env, ip) {
  const limit = parseInt(env.DAILY_LIMIT || "50", 10);
  const { allowed, remaining } = await checkRateLimit(env, ip, "groq-whisper", limit);

  if (!allowed) {
    return errorResponse("Daily demo limit reached.", 429);
  }

  // Forward the multipart form data as-is
  const resp = await fetch("https://api.groq.com/openai/v1/audio/transcriptions", {
    method: "POST",
    headers: {
      Authorization: `Bearer ${env.GROQ_API_KEY}`,
    },
    body: request.body,
  });

  const data = await resp.text();
  return new Response(data, {
    status: resp.status,
    headers: {
      ...CORS_HEADERS,
      "Content-Type": "application/json",
      "X-RateLimit-Remaining": String(remaining),
    },
  });
}

// ---- Router ----

export default {
  async fetch(request, env, ctx) {
    const url = new URL(request.url);
    const ip = request.headers.get("CF-Connecting-IP") || "unknown";

    // CORS preflight — no analytics for OPTIONS
    if (request.method === "OPTIONS") {
      return new Response(null, { status: 204, headers: CORS_HEADERS });
    }

    const deviceId = request.headers.get("X-Device-ID") || null;
    const startTime = Date.now();
    let response;

    // Routes
    switch (url.pathname) {
      case "/health":
        response = corsResponse(JSON.stringify({ status: "ok" }));
        break;

      case "/api/deepgram/token":
        if (request.method !== "GET") { response = errorResponse("Method not allowed", 405); break; }
        response = await handleDeepgramToken(env, ip);
        break;

      case "/api/groq/chat":
        if (request.method !== "POST") { response = errorResponse("Method not allowed", 405); break; }
        response = await handleGroqChat(request, env, ip);
        break;

      case "/api/groq/whisper":
        if (request.method !== "POST") { response = errorResponse("Method not allowed", 405); break; }
        response = await handleGroqWhisper(request, env, ip);
        break;

      case "/api/heartbeat":
        if (request.method !== "POST") { response = errorResponse("Method not allowed", 405); break; }
        response = await handleHeartbeat(request, env, ip);
        break;

      default:
        response = errorResponse("Not found", 404);
    }

    // Fire-and-forget analytics (does not delay the response)
    const durationMs = Date.now() - startTime;
    ctx.waitUntil(
      hashIP(ip).then(ipHash =>
        logAnalytics(env, {
          ipHash,
          endpoint: url.pathname,
          method: request.method,
          status: response.status,
          durationMs,
          deviceId,
        })
      )
    );

    return response;
  },
};
